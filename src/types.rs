#[cfg(not(feature = "parse-comments"))]
pub type Comment = ();
#[cfg(feature = "parse-comments")]
pub type Comment = String;

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    RealNumber(f64),
    #[cfg(feature = "string-value")]
    String(String),
}
impl Literal {
    pub fn as_real_number(&self) -> Option<f64> {
        match self {
            Literal::RealNumber(rn) => Some(*rn),
            #[cfg(feature = "string-value")]
            _ => None,
        }
    }
    #[cfg(feature = "string-value")]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Literal::String(string) => Some(string),
            _ => None,
        }
    }
}
impl From<i32> for Literal {
    fn from(from: i32) -> Self {
        Self::RealNumber(from as f64)
    }
}
impl From<u32> for Literal {
    fn from(from: u32) -> Self {
        Self::RealNumber(from as f64)
    }
}
impl From<f64> for Literal {
    fn from(from: f64) -> Self {
        Self::RealNumber(from)
    }
}

#[cfg(feature = "string-value")]
impl From<String> for Literal {
    fn from(from: String) -> Self {
        Self::String(from)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RealValue {
    Literal(Literal),
    #[cfg(any(feature = "parse-parameters", feature = "parse-expressions"))]
    Expression(expressions::Expression),
    #[cfg(feature = "optional-value")]
    None,
}
impl Default for RealValue {
    fn default() -> Self {
        Self::from(0.)
    }
}
impl<T: Into<Literal>> From<T> for RealValue {
    fn from(from: T) -> Self {
        RealValue::Literal(from.into())
    }
}

#[cfg(any(feature = "parse-parameters", feature = "parse-expressions"))]
pub(crate) mod expressions {
    use super::{Literal, RealValue};
    use crate::Error;
    use either::Either;

    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    pub(crate) type ExprItem = Either<Operator, Literal>;
    pub(crate) type ExprInner = Vec<ExprItem>;

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum OpType {
        Unary,
        Binary,
    }
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum Associativity {
        Left,
        #[cfg(feature = "parse-parameters")]
        Right,
    }
    #[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
    pub enum Precedence {
        Group1,
        Group2,
        Group3,
        #[cfg(feature = "parse-parameters")]
        Group4,
        Group5,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum Operator {
        // Binary operators
        Add,
        Substract,
        Multiply,
        Divide,
        Power,

        And,
        Or,
        Xor,

        Modulus,

        // Unary operators
        Cos,
        Sin,
        Tan,
        ACos,
        ASin,
        ATan, // Atan is kind of binary

        Abs,
        Exp,
        Fix,
        Fup,
        Ln,
        Round,
        Sqrt,

        #[cfg(feature = "parse-parameters")]
        GetParameter,
    }
    impl Operator {
        pub fn op_type(&self) -> OpType {
            match self {
                Self::Add
                | Self::Substract
                | Self::Multiply
                | Self::Divide
                | Self::Modulus
                | Self::Power
                | Self::And
                | Self::Or
                | Self::Xor
                | Self::ATan => OpType::Binary,
                Self::Cos
                | Self::Sin
                | Self::Tan
                | Self::ACos
                | Self::ASin
                | Self::Abs
                | Self::Exp
                | Self::Fix
                | Self::Fup
                | Self::Ln
                | Self::Round
                | Self::Sqrt => OpType::Unary,
                #[cfg(feature = "parse-parameters")]
                Self::GetParameter => OpType::Unary,
            }
        }

        pub fn associativity(&self) -> Associativity {
            #[allow(clippy::match_single_binding)]
            match self {
                #[cfg(feature = "parse-parameters")]
                Self::GetParameter => Associativity::Right,
                _ => Associativity::Left,
            }
        }

        pub fn precedence(&self) -> Precedence {
            match *self {
                Self::Add | Self::Substract | Self::And | Self::Or | Self::Xor => {
                    Precedence::Group1
                }
                Self::Multiply | Self::Divide | Self::Modulus => Precedence::Group2,
                Self::Power => Precedence::Group3,
                #[cfg(feature = "parse-parameters")]
                Self::GetParameter => Precedence::Group4,
                Self::Cos
                | Self::Sin
                | Self::Tan
                | Self::ACos
                | Self::ASin
                | Self::ATan
                | Self::Abs
                | Self::Exp
                | Self::Fix
                | Self::Fup
                | Self::Ln
                | Self::Round
                | Self::Sqrt => Precedence::Group5,
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    pub struct Expression(pub ExprInner);
    impl Expression {
        pub fn resolve(&self, _cbk: &mut dyn FnMut(Literal) -> Literal) -> Result<Literal, Error> {
            todo!()
        }
    }

    impl Default for Expression {
        fn default() -> Self {
            Self(Vec::new())
        }
    }

    impl core::ops::Deref for Expression {
        type Target = ExprInner;
        fn deref(&self) -> &ExprInner {
            &self.0
        }
    }

    impl core::ops::DerefMut for Expression {
        fn deref_mut(&mut self) -> &mut ExprInner {
            &mut self.0
        }
    }

    impl From<ExprInner> for Expression {
        fn from(infix: ExprInner) -> Self {
            Self(infix)
        }
    }

    impl From<Operator> for Either<Operator, Literal> {
        fn from(from: Operator) -> Self {
            Self::Left(from)
        }
    }
    impl From<Literal> for Either<Operator, Literal> {
        fn from(from: Literal) -> Self {
            Self::Right(from)
        }
    }

    impl From<Expression> for Either<Literal, Expression> {
        fn from(from: Expression) -> Self {
            Self::Right(from)
        }
    }
    impl From<Literal> for Either<Literal, Expression> {
        fn from(from: Literal) -> Self {
            Self::Left(from)
        }
    }

    impl From<Expression> for RealValue {
        fn from(from: Expression) -> RealValue {
            RealValue::Expression(from)
        }
    }
}
