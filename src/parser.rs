//! The ebnf representatio following [https://bottlecaps.de/rr/ui]'s syntax
//! ```ebnf
//! line       ::= '/'? ( [Nn] [0-9]+ )?
//!                ( [a-zA-Z] real_value? | '#' real_value '=' real_value? | '(' [^)] ')' )*
//!                ( '*' [0-9]+ /* 0 to 255 */ )?
//!                ( ';' [^\n]* )? '\n'
//! real_value ::= '#'* ( real_number
//!                     | '"' [^"] '"'
//!                     | 'atan' expression '/' expression
//!                     | ( 'abs' | 'acos' | 'asin' | 'cos' | 'exp' | 'fix' | 'fup' | 'ln' | 'round' | 'sin' | 'sqrt' | 'tan' ) expression )
//! expression ::= '[' real_value ( ( '**' | '/' | 'mod' | '*' | 'and' | 'xor' | '-' | 'or' | '+' ) real_value )* ']'
//! real_number ::= ( '+' | '-' )? ( [0-9]+ ( '.' [0-9]* )? | '.' [0-9]+ )
//! ```
//!
mod values;

#[cfg(feature = "parse-expressions")]
mod expressions;

#[cfg(test)]
mod test;

use core::marker::Unpin;
use futures::stream::{self, StreamExt};

use crate::{
    stream::{MyStreamExt, PushBackable},
    types::Comment,
    utils::skip_whitespaces,
    Error, GCode,
};

use values::parse_number;

#[cfg(not(feature = "parse-expressions"))]
use values::parse_real_value;

#[cfg(feature = "parse-expressions")]
use expressions::parse_real_value;

#[derive(PartialEq, Debug, Clone, Copy)]
enum AsyncParserState {
    Start(bool),
    LineNumberOrSegment,
    Segment,
    ErrorRecovery,
    #[cfg(all(feature = "parse-trailing-comment", feature = "parse-checksum"))]
    EoLOrTrailingComment,
    #[cfg(any(feature = "parse-trailing-comment", feature = "parse-checksum"))]
    EndOfLine,
}

#[cfg(all(feature = "parse-trailing-comment", not(feature = "parse-comments")))]
async fn parse_eol_comment<S>(input: &mut S) -> Option<Comment>
where
    S: stream::Stream<Item = u8>
        + core::marker::Unpin
        + PushBackable<Item = <S as stream::Stream>::Item>,
{
    loop {
        let b = input.next().await?;
        match b {
            b'\r' | b'\n' => {
                input.push_back(b);
                break Some(());
            }
            _ => {}
        }
    }
}

#[cfg(all(feature = "parse-trailing-comment", feature = "parse-comments"))]
async fn parse_eol_comment<S>(input: &mut S) -> Option<Result<Comment, Error>>
where
    S: stream::Stream<Item = u8>
        + core::marker::Unpin
        + PushBackable<Item = <S as stream::Stream>::Item>,
{
    let mut v = Vec::new();
    loop {
        let b = input.next().await?;
        match b {
            b'\r' | b'\n' => {
                input.push_back(b);
                break Some(String::from_utf8(v).map_err(|_| Error::InvalidUTF8String));
            }
            b => v.push(b),
        }
    }
}

#[cfg(not(feature = "parse-comments"))]
async fn parse_inline_comment<S>(input: &mut S) -> Option<Result<Comment, Error>>
where
    S: stream::Stream<Item = u8>
        + core::marker::Unpin
        + PushBackable<Item = <S as stream::Stream>::Item>,
{
    loop {
        match input.next().await? {
            b'\\' => {
                input.next().await?;
            }
            b'(' => break Some(Err(Error::UnexpectedByte(b'('))),
            b')' => break Some(Ok(())),
            _ => {}
        }
    }
}

#[cfg(feature = "parse-comments")]
async fn parse_inline_comment<S>(input: &mut S) -> Option<Result<Comment, Error>>
where
    S: stream::Stream<Item = u8>
        + core::marker::Unpin
        + PushBackable<Item = <S as stream::Stream>::Item>,
{
    let mut v = Vec::new();
    loop {
        match input.next().await? {
            b'\\' => {
                v.push(input.next().await?);
            }
            b'(' => break Some(Err(Error::UnexpectedByte(b'('))),
            b')' => break Some(String::from_utf8(v).map_err(|_| Error::InvalidUTF8String)),
            b => v.push(b),
        }
    }
}

// use a different struct to compute checksum
#[cfg(not(feature = "parse-checksum"))]
use crate::stream::pushback::PushBack;
#[cfg(feature = "parse-checksum")]
type PushBack<T> = crate::stream::xorsum_pushback::XorSumPushBack<T>;

async fn parse_eol<S: stream::Stream<Item = u8> + Unpin>(
    state: &mut AsyncParserState,
    input: &mut PushBack<S>,
) -> Option<Result<GCode, Error>> {
    Some(loop {
        match input.next().await? {
            b'\r' | b'\n' => {
                *state = AsyncParserState::Start(true);
                #[cfg(feature = "parse-checksum")]
                {
                    input.reset_sum(0);
                }
                break Ok(GCode::Execute);
            }
            b' ' => {}
            b => break Err(Error::UnexpectedByte(b)),
        }
    })
}

pub struct Parser<S>
where
    S: stream::Stream<Item = u8> + core::marker::Unpin,
{
    input: PushBack<S>,
    state: AsyncParserState,
}

impl<S> Parser<S>
where
    S: stream::Stream<Item = u8> + Unpin,
{
    pub fn new(input: S) -> Self {
        Self {
            #[cfg(feature = "parse-checksum")]
            input: input.xor_summed_push_backable(0),
            #[cfg(not(feature = "parse-checksum"))]
            input: input.push_backable(),
            state: AsyncParserState::Start(true),
        }
    }
    pub async fn next(self) -> Option<(Result<GCode, Error>, Self)> {
        let Parser {
            mut input,
            mut state,
        } = self;

        let res = loop {
            let b = input.next().await?;
            // println!("{:?}: {:?}", state, char::from(b));
            match state {
                AsyncParserState::Start(ref mut first_byte) => match b {
                    b'\r' | b'\n' => {
                        *first_byte = true; /* ignore empty new lines */
                        #[cfg(feature = "parse-checksum")]
                        {
                            input.reset_sum(0);
                        }
                    }
                    b'/' if *first_byte => {
                        state = AsyncParserState::LineNumberOrSegment;
                        break Ok(GCode::BlockDelete);
                    }
                    b' ' => {
                        *first_byte = false;
                    }
                    _ => {
                        input.push_back(b);
                        state = AsyncParserState::LineNumberOrSegment
                    }
                },
                AsyncParserState::LineNumberOrSegment => match b.to_ascii_lowercase() {
                    b'n' => {
                        skip_whitespaces(&mut input).await?;
                        break match parse_number(&mut input).await? {
                            Ok((n, ord)) => {
                                if ord == 1 {
                                    Err(Error::UnexpectedByte(input.next().await?))
                                } else if ord > 10000 {
                                    Err(Error::NumberOverflow)
                                } else {
                                    state = AsyncParserState::Segment;
                                    Ok(GCode::LineNumber(n))
                                }
                            }
                            Err(e) => Err(e),
                        };
                    }
                    _ => {
                        input.push_back(b);
                        state = AsyncParserState::Segment;
                    }
                },
                AsyncParserState::Segment => match b.to_ascii_lowercase() {
                    b' ' => {}
                    letter @ b'a'..=b'z' => {
                        skip_whitespaces(&mut input).await?;
                        break match parse_real_value(&mut input).await? {
                            Ok(rv) => {
                                // println!("word({:?}, {:?})", letter as char, rv);
                                Ok(GCode::Word(letter.into(), rv))
                            }
                            Err(e) => Err(e),
                        };
                    }
                    b'\r' | b'\n' => {
                        input.push_back(b);
                        break parse_eol(&mut state, &mut input).await?;
                    }
                    // param support feature
                    #[cfg(feature = "parse-parameters")]
                    b'#' => {
                        skip_whitespaces(&mut input).await?;
                        let param_id = match parse_real_value(&mut input).await? {
                            #[cfg(feature = "optional-value")]
                            Ok(crate::types::RealValue::None) => {
                                break Err(Error::UnexpectedByte((&mut input).next().await?))
                            }
                            Ok(id) => id,
                            Err(e) => break Err(e),
                        };
                        skip_whitespaces(&mut input).await?;
                        let b = (&mut input).next().await?;
                        if b'=' != b {
                            break Err(Error::UnexpectedByte(b));
                        }
                        skip_whitespaces(&mut input).await?;
                        let value = match parse_real_value(&mut input).await? {
                            Ok(id) => id,
                            Err(e) => break Err(e),
                        };
                        break Ok(GCode::ParameterSet(param_id, value));
                    }
                    // checksum support feature
                    #[cfg(feature = "parse-checksum")]
                    b'*' => {
                        let sum = input.sum() ^ b'*';
                        skip_whitespaces(&mut input).await?;
                        match parse_number(&mut input).await? {
                            Ok((n, _)) => {
                                // println!("{} {}", sum, n);
                                if n >= 256 {
                                    break Err(Error::NumberOverflow);
                                } else if (n as u8) != sum {
                                    break Err(Error::BadChecksum(sum));
                                } else {
                                    skip_whitespaces(&mut input).await?;
                                    #[cfg(not(feature = "parse-trailing-comment"))]
                                    {
                                        state = AsyncParserState::EndOfLine;
                                    }
                                    #[cfg(feature = "parse-trailing-comment")]
                                    {
                                        state = AsyncParserState::EoLOrTrailingComment;
                                    }
                                }
                            }
                            Err(e) => break Err(e),
                        }
                    }
                    // comment support features
                    #[cfg(not(feature = "parse-comments"))]
                    b'(' => {
                        if let Err(e) = parse_inline_comment(&mut input).await? {
                            break Err(e);
                        }
                    }
                    #[cfg(feature = "parse-comments")]
                    b'(' => break parse_inline_comment(&mut input).await?.map(GCode::Comment),
                    #[cfg(all(
                        feature = "parse-trailing-comment",
                        not(feature = "parse-comments")
                    ))]
                    b';' => {
                        parse_eol_comment(&mut input).await?;
                        state = AsyncParserState::EndOfLine;
                    }
                    #[cfg(all(feature = "parse-trailing-comment", feature = "parse-comments"))]
                    b';' => {
                        break match parse_eol_comment(&mut input).await? {
                            Ok(s) => {
                                state = AsyncParserState::EndOfLine;
                                Ok(GCode::Comment(s))
                            }
                            Err(e) => Err(e),
                        }
                    }
                    _ => break Err(Error::UnexpectedByte(b)),
                },
                #[cfg(all(
                    feature = "parse-trailing-comment",
                    not(feature = "parse-comments"),
                    feature = "parse-checksum"
                ))]
                AsyncParserState::EoLOrTrailingComment => match b {
                    b';' => parse_eol_comment(&mut input).await?,
                    _ => {
                        input.push_back(b);
                        break parse_eol(&mut state, &mut input).await?;
                    }
                },
                #[cfg(all(
                    feature = "parse-trailing-comment",
                    feature = "parse-comments",
                    feature = "parse-checksum"
                ))]
                AsyncParserState::EoLOrTrailingComment => match b {
                    b';' => {
                        break match parse_eol_comment(&mut input).await? {
                            Ok(s) => {
                                state = AsyncParserState::EndOfLine;
                                Ok(GCode::Comment(s))
                            }
                            Err(e) => Err(e),
                        }
                    }
                    _ => {
                        input.push_back(b);
                        break parse_eol(&mut state, &mut input).await?;
                    }
                },
                #[cfg(any(feature = "parse-trailing-comment", feature = "parse-checksum"))]
                AsyncParserState::EndOfLine => {
                    input.push_back(b);
                    break parse_eol(&mut state, &mut input).await?;
                }
                AsyncParserState::ErrorRecovery => match b {
                    b'\r' | b'\n' => {
                        input.push_back(b);
                        break parse_eol(&mut state, &mut input).await?;
                    }
                    _ => {}
                },
            }
        };
        if res.is_err() {
            state = AsyncParserState::ErrorRecovery;
        }
        Some((res, Self { input, state }))
    }
}
