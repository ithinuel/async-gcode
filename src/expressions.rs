use crate::lexer::{Operator, Token};
#[cfg(feature = "no_std")]
use crate::std::boxed::Box;
use crate::std::convert::TryFrom;
use crate::std::mem;
#[cfg(feature = "no_std")]
use crate::std::vec::Vec;
use crate::utils::Either;
use crate::RealValue;

#[derive(Debug, PartialEq)]
pub enum Expression {
    Abs(RealValue),
    Add(RealValue, RealValue),
    And(RealValue, RealValue),
    ACos(RealValue),
    ASin(RealValue),
    ATan(RealValue, RealValue),
    Cos(RealValue),
    Div(RealValue, RealValue),
    Pow(RealValue, RealValue),
    Fix(RealValue),
    Fup(RealValue),
    Ln(RealValue),
    Mod(RealValue, RealValue),
    Mul(RealValue, RealValue),
    Or(RealValue, RealValue),
    Round(RealValue),
    Sin(RealValue),
    Sqrt(RealValue),
    Sub(RealValue, RealValue),
    Tan(RealValue),
    Xor(RealValue, RealValue),
}
impl TryFrom<Vec<Either<RealValue, Token>>> for Expression {
    type Error = ();
    fn try_from(mut expr: Vec<Either<RealValue, Token>>) -> Result<Self, Self::Error> {
        for precedence in 0..=2 {
            let mut idx = 1;
            loop {
                if idx >= expr.len() {
                    break;
                }
                let prio = match &expr[idx] {
                    Either::Left(_) => unimplemented!(),
                    Either::Right(tok) => match tok {
                        Token::Power => 0,
                        Token::Times | Token::Slash | Token::Operator(Operator::Mod) => 1,
                        _ => 2,
                    },
                };
                if prio == precedence {
                    let prev = idx - 1;
                    let lhs = match expr.remove(prev) {
                        Either::Left(val) => val,
                        Either::Right(_) => unimplemented!(),
                    };
                    let rhs = match expr.remove(idx) {
                        Either::Left(val) => val,
                        Either::Right(_) => unimplemented!(),
                    };
                    let exp = Either::Left(RealValue::Expression(Box::new(
                        match match &expr[prev] {
                            Either::Left(_) => unreachable!(),
                            Either::Right(op) => op,
                        } {
                            Token::Plus => Expression::Add(lhs, rhs),
                            Token::Minus => Expression::Sub(lhs, rhs),
                            Token::Times => Expression::Mul(lhs, rhs),
                            Token::Slash => Expression::Div(lhs, rhs),
                            Token::Power => Expression::Pow(lhs, rhs),
                            Token::Operator(Operator::Mod) => Expression::Mod(lhs, rhs),
                            Token::Operator(Operator::And) => Expression::And(lhs, rhs),
                            Token::Operator(Operator::Or) => Expression::Or(lhs, rhs),
                            Token::Operator(Operator::Xor) => Expression::Xor(lhs, rhs),
                            _ => unimplemented!(),
                        },
                    )));
                    mem::replace(&mut expr[prev], exp);
                } else {
                    idx += 2;
                }
            }
        }
        debug_assert!(expr.len() == 1);
        match expr.pop() {
            Some(Either::Left(RealValue::Expression(op))) => Ok(*op),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod test {}
