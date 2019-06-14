//! This crate implements a GCode (RS-274) parser.
//! The default dialect is taken from NIST's [RS274/NGC interpreter version 3]( https://www.nist.gov/publications/nist-rs274ngc-interpreter-version-3?pub_id=823374).
//!
//! It might be interesting to have a look at ISO 6983 and/or ISO 14649.
#![cfg_attr(feature = "no_std", no_std)]

#[cfg(feature = "no_std")]
extern crate alloc;

#[cfg(feature = "parse-expressions")]
mod expressions;
mod lexer;
mod parser;
#[cfg(feature = "no_std")]
mod std;
mod utils;

#[cfg(all(
    feature = "no_std",
    any(feature = "parse-expressions", feature = "parse-parameters")
))]
use crate::std::boxed::Box;
#[cfg(all(feature = "no_std", feature = "parse-comments"))]
use crate::std::string::String;
#[cfg(not(feature = "no_std"))]
use std;

#[cfg(feature = "parse-expressions")]
pub use expressions::Expression;
pub use lexer::Token;
pub use parser::Parser;

#[derive(Debug, PartialEq)]
pub enum Error {
    UnexpectedChar(lexer::State, char),
    UnexpectedToken(parser::State, lexer::Token),
}

#[cfg(feature = "fixed-point")]
type RealNumber = i32;
#[cfg(not(feature = "fixed-point"))]
type RealNumber = f32;

#[derive(Debug, PartialEq)]
pub enum RealValue {
    RealNumber(RealNumber),
    #[cfg(feature = "parse-parameters")]
    ParameterGet(Box<RealValue>),
    #[cfg(feature = "parse-expressions")]
    Expression(Box<Expression>),
}
impl RealValue {
    fn build_real_number(
        negativ: bool,
        integer: u32,
        decimal: u32,
        decimal_order: u32,
    ) -> RealValue {
        let (one, neg_one) = (1., -1.);

        let (integer, decimal, decimal_order, decimal_point) =
            (integer as f32, decimal as f32, decimal_order as f32, 1.);

        let v = if negativ { neg_one } else { one }
            * (integer * decimal_point + decimal * (decimal_point / decimal_order));
        RealValue::RealNumber(v)
    }
}
impl Default for RealValue {
    fn default() -> Self {
        RealValue::RealNumber(0.)
    }
}

#[derive(Debug, PartialEq)]
pub enum GCode {
    BlockDelete,
    LineNumber(u32),
    #[cfg(feature = "parse-comments")]
    Comment(String),
    Word(char, RealValue),
    #[cfg(feature = "parse-parameters")]
    ParameterSet(RealValue, RealValue),
    Execute,
}
