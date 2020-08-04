//! This crate implements a GCode (RS-274) parser.
//! The default dialect is taken from NIST's [RS274/NGC interpreter version 3](https://www.nist.gov/publications/nist-rs274ngc-interpreter-version-3?pub_id=823374).
//!
//! It might be interesting to have a look at ISO 6983 and/or ISO 14649.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(
    not(feature = "std"),
    any(feature = "parse-parameters", feature = "parse-comments")
))]
extern crate alloc;

mod stream;
mod types;
mod utils;

mod parser;

pub use parser::Parser;
pub use types::RealValue;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    UnexpectedByte(u8),
    NumberOverflow,
    BadNumberFormat,
    #[cfg(any(feature = "parse-comments", feature = "string-value"))]
    InvalidUTF8String,
    #[cfg(feature = "parse-checksum")]
    BadChecksum(u8),
    #[cfg(feature = "parse-expressions")]
    InvalidExpression,
}

#[derive(Debug, PartialEq, Clone)]
pub enum GCode {
    BlockDelete,
    LineNumber(u32),
    #[cfg(feature = "parse-comments")]
    Comment(std::string::String),
    Word(char, RealValue),
    #[cfg(feature = "parse-parameters")]
    ParameterSet(RealValue, RealValue),
    Execute,
}
