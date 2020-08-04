#![allow(clippy::useless_conversion)]

//! we need/want to store a value of up to 10^6 with a precision of 10^-5.
//! Fix point arithmetics would require a 14bit fractional part plus a 18bit integer part.
//!
//! Comparing f64 sqrt and few implementation of the sqrt approximation algorithm using the `fixed`
//! crate shows that there's no subtantial benefit in terms of flash size from using the fixed
//! point arithmetics.

use core::marker;
use futures::{Stream, StreamExt};

#[cfg(all(
    not(feature = "std"),
    any(feature = "parse-parameters", feature = "parse-expressions")
))]
use alloc::vec::Vec;

use crate::{stream::PushBackable, types::Literal, utils::skip_whitespaces, Error};

#[cfg(not(feature = "parse-expressions"))]
use crate::types::RealValue;

#[cfg(any(feature = "parse-parameters", feature = "parse-expressions"))]
pub use crate::types::expressions::{Expression, Operator};

pub(crate) async fn parse_number<S>(input: &mut S) -> Option<Result<(u32, u32), Error>>
where
    S: Stream<Item = u8> + marker::Unpin + PushBackable<Item = <S as Stream>::Item>,
{
    let mut n = 0;
    let mut order = 1;
    let res = loop {
        let b = input.next().await?;
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

async fn parse_real_literal<S>(input: &mut S) -> Option<Result<f64, Error>>
where
    S: Stream<Item = u8> + marker::Unpin + PushBackable<Item = <S as Stream>::Item>,
{
    // extract sign: default to positiv
    let mut b = input.next().await?;

    let mut negativ = false;

    if b == b'-' || b == b'+' {
        negativ = b == b'-';

        // skip spaces after the sign
        skip_whitespaces(input).await?;
        b = input.next().await?;
    }

    // parse integer part
    let int = if b != b'.' {
        input.push_back(b);
        // if not a decimal point, there must be an integer
        match parse_number(input).await? {
            Ok((v, _)) => {
                // skip spaces after integer part
                skip_whitespaces(input).await?;
                b = input.next().await?;
                Some(v)
            }
            Err(e) => return Some(Err(e)),
        }
    } else {
        None
    };

    // parse decimal part: mandatory if integer part is abscent
    let dec = if b == b'.' {
        // skip spaces after decimal point
        skip_whitespaces(input).await?;
        match parse_number(input).await? {
            Ok((decimal, order)) => Some((decimal, order)),
            Err(e) => return Some(Err(e)),
        }
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
        Err(Error::BadNumberFormat)
    } else {
        let int = int.map(f64::from).unwrap_or(0.);
        let (dec, ord) = dec
            .map(|(dec, ord)| (dec.into(), ord.into()))
            .unwrap_or((0., 1.));
        Ok((if negativ { -1. } else { 1. }) * (int + dec / ord))
    };
    Some(res)
}

#[cfg(feature = "string-value")]
async fn parse_string_literal<S>(input: &mut S) -> Option<Result<String, Error>>
where
    S: Stream<Item = u8> + marker::Unpin + PushBackable<Item = <S as Stream>::Item>,
{
    // we cannot use take_until(…).collect() because we need to distinguish input's end of stream
    // from take_until(…) end of stream

    let mut array = vec![];
    loop {
        match input.next().await? {
            b'"' => break,
            b'\\' => {
                array.push(input.next().await?);
            }
            b => array.push(b),
        }
    }

    match String::from_utf8(array) {
        Ok(string) => Some(Ok(string)),
        Err(_) => Some(Err(Error::InvalidUTF8String)),
    }
}

pub(crate) async fn parse_literal<S>(input: &mut S) -> Option<Result<Literal, Error>>
where
    S: Stream<Item = u8> + marker::Unpin + PushBackable<Item = <S as Stream>::Item>,
{
    let b = input.next().await?;
    Some(match b {
        b'+' | b'-' | b'.' | b'0'..=b'9' => {
            input.push_back(b);
            parse_real_literal(input).await?.map(Literal::from)
        }
        #[cfg(feature = "string-value")]
        b'"' => parse_string_literal(input).await?.map(Literal::from),
        _ => Err(Error::UnexpectedByte(b)),
    })
}

#[cfg(not(feature = "parse-expressions"))]
pub(crate) async fn parse_real_value<S>(input: &mut S) -> Option<Result<RealValue, Error>>
where
    S: Stream<Item = u8> + marker::Unpin + PushBackable<Item = <S as Stream>::Item>,
{
    let b = input.next().await?;
    // println!("real value: {:?}", b as char);

    let res = match b {
        b'+' | b'-' | b'.' | b'0'..=b'9' => {
            input.push_back(b);
            parse_literal(input).await?.map(RealValue::from)
        }
        #[cfg(feature = "string-value")]
        b'"' => {
            input.push_back(b);
            parse_literal(input).await?.map(RealValue::from)
        }
        #[cfg(feature = "parse-parameters")]
        b'#' => {
            let mut n = 1;
            let literal = loop {
                skip_whitespaces(input).await?;
                let b = input.next().await?;
                if b != b'#' {
                    input.push_back(b);

                    break match parse_literal(input).await? {
                        Ok(literal) => literal,
                        Err(e) => return Some(Err(e)),
                    };
                }
                n += 1;
            };

            let vec: Vec<_> = core::iter::once(literal.into())
                .chain(core::iter::repeat(Operator::GetParameter.into()).take(n))
                .collect();
            Ok(Expression::from(vec).into())
        }
        #[cfg(feature = "optional-value")]
        b => {
            input.push_back(b);
            Ok(RealValue::None)
        }
        #[cfg(not(feature = "optional-value"))]
        b => Err(Error::UnexpectedByte(b)),
    };
    Some(res)
}
