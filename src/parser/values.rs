#![allow(clippy::useless_conversion)]

//! we need/want to store a value of up to 10^6 with a precision of 10^-5.
//! Fix point arithmetics would require a 14bit fractional part plus a 18bit integer part.
//!
//! Comparing f64 sqrt and few implementation of the sqrt approximation algorithm using the `fixed`
//! crate shows that there's no subtantial benefit in terms of flash size from using the fixed
//! point arithmetics.

use futures::{Stream, StreamExt};

#[cfg(all(not(feature = "std"), feature = "string-value"))]
use alloc::string::String;

use crate::{
    stream::PushBackable,
    types::{Literal, ParseResult},
    utils::skip_whitespaces,
    Error,
};

#[cfg(not(feature = "parse-expressions"))]
use crate::types::RealValue;

#[cfg(any(feature = "parse-parameters", feature = "parse-expressions"))]
pub use crate::types::expressions::{Expression, Operator};

pub(crate) async fn parse_number<S, E>(input: &mut S) -> Option<Result<(u32, u32), E>>
where
    S: Stream<Item = Result<u8, E>> + Unpin + PushBackable<Item = u8>,
    E: core::fmt::Debug,
{
    let mut n = 0;
    let mut order = 1;
    let res = loop {
        let b = match input.next().await? {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };
        match b {
            b'0'..=b'9' => {
                let digit = u32::from(b - b'0');
                n = n * 10 + digit;
                order *= 10;
            }
            _ => {
                input.push_back(b);
                break Ok((n, order));
            }
        }
    };
    Some(res)
}

async fn parse_real_literal<S, E>(input: &mut S) -> Option<ParseResult<f64, E>>
where
    S: Stream<Item = Result<u8, E>> + Unpin + PushBackable<Item = u8>,
    E: core::fmt::Debug,
{
    // extract sign: default to positiv
    let mut b = try_result!(input.next());

    let mut negativ = false;

    if b == b'-' || b == b'+' {
        negativ = b == b'-';

        // skip spaces after the sign
        try_result!(skip_whitespaces(input));
        b = try_result!(input.next());
    }

    // parse integer part
    let int = if b != b'.' {
        input.push_back(b);
        // if not a decimal point, there must be an integer
        let (v, _) = try_result!(parse_number(input));
        // skip spaces after integer part
        try_result!(skip_whitespaces(input));
        b = try_result!(input.next());
        Some(v)
    } else {
        None
    };

    // parse decimal part: mandatory if integer part is abscent
    let dec = if b == b'.' {
        // skip spaces after decimal point
        try_result!(skip_whitespaces(input));
        Some(try_result!(parse_number(input)))
    } else {
        input.push_back(b);
        None
    };

    //println!(
    //"literal done: {} {:?} {:?}",
    //if negativ { '-' } else { '+' },
    //int,
    //dec
    //);

    let res = if int.is_none() && dec.is_none() {
        ParseResult::Parsing(Error::BadNumberFormat.into())
    } else {
        let int = int.map(f64::from).unwrap_or(0.);
        let (dec, ord) = dec
            .map(|(dec, ord)| (dec.into(), ord.into()))
            .unwrap_or((0., 1.));
        ParseResult::Ok((if negativ { -1. } else { 1. }) * (int + dec / ord))
    };
    Some(res)
}

#[cfg(feature = "string-value")]
async fn parse_string_literal<S, E>(input: &mut S) -> Option<ParseResult<String, E>>
where
    S: Stream<Item = Result<u8, E>> + Unpin + PushBackable<Item = u8>,
    E: core::fmt::Debug,
{
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    // we cannot use take_until(…).collect() because we need to distinguish input's end of stream
    // from take_until(…) end of stream

    let mut array = Vec::new();
    loop {
        match try_result!(input.next()) {
            b'"' => break,
            b'\\' => {
                array.push(try_result!(input.next()));
            }
            b => array.push(b),
        }
    }

    match String::from_utf8(array) {
        Ok(string) => Some(ParseResult::Ok(string)),
        Err(_) => Some(Error::InvalidUTF8String.into()),
    }
}

pub(crate) async fn parse_literal<S, E>(input: &mut S) -> Option<ParseResult<Literal, E>>
where
    S: Stream<Item = Result<u8, E>> + Unpin + PushBackable<Item = u8>,
    E: core::fmt::Debug,
{
    let b = try_result!(input.next());
    Some(match b {
        b'+' | b'-' | b'.' | b'0'..=b'9' => {
            input.push_back(b);
            ParseResult::Ok(Literal::from(try_parse!(parse_real_literal(input))))
        }
        #[cfg(feature = "string-value")]
        b'"' => ParseResult::Ok(Literal::from(try_parse!(parse_string_literal(input)))),
        _ => Error::UnexpectedByte(b).into(),
    })
}

#[cfg(not(feature = "parse-expressions"))]
pub(crate) async fn parse_real_value<S, E>(input: &mut S) -> Option<ParseResult<RealValue, E>>
where
    S: Stream<Item = Result<u8, E>> + Unpin + PushBackable<Item = u8>,
    E: core::fmt::Debug,
{
    let b = try_result!(input.next());
    // println!("real value: {:?}", b as char);

    let res = match b {
        b'+' | b'-' | b'.' | b'0'..=b'9' => {
            input.push_back(b);
            ParseResult::Ok(RealValue::from(try_parse!(parse_literal(input))))
        }
        #[cfg(feature = "string-value")]
        b'"' => {
            input.push_back(b);
            ParseResult::Ok(RealValue::from(try_parse!(parse_literal(input))))
        }
        #[cfg(feature = "parse-parameters")]
        b'#' => {
            #[cfg(not(feature = "std"))]
            use alloc::vec::Vec;

            let mut n = 1;
            let literal = loop {
                try_result!(skip_whitespaces(input));
                let b = try_result!(input.next());
                if b != b'#' {
                    input.push_back(b);

                    break try_parse!(parse_literal(input));
                }
                n += 1;
            };

            let vec: Vec<_> = core::iter::once(literal.into())
                .chain(core::iter::repeat(Operator::GetParameter.into()).take(n))
                .collect();
            ParseResult::Ok(Expression::from(vec).into())
        }
        #[cfg(feature = "optional-value")]
        b => {
            input.push_back(b);
            ParseResult::Ok(RealValue::None)
        }
        #[cfg(not(feature = "optional-value"))]
        b => Error::UnexpectedByte(b).into(),
    };
    Some(res)
}
