use crate::std::mem;
use crate::utils::Stack;
use crate::{lexer, Error, GCode, RealValue, Token};

#[cfg(all(
    feature = "no_std",
    any(feature = "parse-expressions", feature = "parse-parameters")
))]
use crate::std::boxed::Box;

#[cfg(feature = "parse-expressions")]
use crate::expressions::Expression;
#[cfg(feature = "parse-expressions")]
use crate::lexer::Operator;
#[cfg(feature = "parse-expressions")]
use crate::std::convert::TryFrom;
#[cfg(feature = "parse-expressions")]
use crate::std::vec::Vec;
#[cfg(feature = "parse-expressions")]
use crate::utils::Either;

#[cfg(feature = "parse-expressions")]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnaryOperator {
    Abs,
    ACos,
    ASin,
    Cos,
    Fix,
    Fup,
    Ln,
    Round,
    Sin,
    Sqrt,
    Tan,
}
#[cfg(feature = "parse-expressions")]
impl TryFrom<Operator> for UnaryOperator {
    type Error = Operator;

    fn try_from(op: Operator) -> Result<Self, Operator> {
        Ok(match op {
            Operator::Abs => UnaryOperator::Abs,
            Operator::ACos => UnaryOperator::ACos,
            Operator::ASin => UnaryOperator::ASin,
            Operator::Cos => UnaryOperator::Cos,
            Operator::Fix => UnaryOperator::Fix,
            Operator::Fup => UnaryOperator::Fup,
            Operator::Ln => UnaryOperator::Ln,
            Operator::Round => UnaryOperator::Round,
            Operator::Sin => UnaryOperator::Sin,
            Operator::Sqrt => UnaryOperator::Sqrt,
            Operator::Tan => UnaryOperator::Tan,
            op => return Err(op),
        })
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RealNumber {
    IntegerOrDot(bool),
    Dot {
        negative: bool,
        integer: u32,
        optional: bool,
    },
    Decimal {
        negative: bool,
        integer: u32,
        optional: bool,
    },
}

#[derive(Debug, PartialEq)]
pub enum State {
    StartOfLine,
    LineNumberOrSegments,
    LineNumber,
    Segments,
    RealValue(bool),
    RealNumber(RealNumber),
    #[cfg(feature = "parse-parameters")]
    ParameterSet(RealValue),
    #[cfg(feature = "parse-expressions")]
    Expression(Vec<Either<RealValue, Token>>),
    #[cfg(feature = "parse-expressions")]
    UnaryExpressionStart(UnaryOperator),
    #[cfg(feature = "parse-expressions")]
    UnaryExpressionEnd(UnaryOperator, RealValue),
    #[cfg(feature = "parse-expressions")]
    ATanComboStart,
    #[cfg(feature = "parse-expressions")]
    ATanComboEnd(RealValue),
    #[cfg(feature = "parse-expressions")]
    ATanComboDividedBy(RealValue),
    #[cfg(feature = "parse-expressions")]
    ATanComboDividedByStart(RealValue),
    #[cfg(feature = "parse-expressions")]
    ATanComboDividedByEnd(RealValue, RealValue),
    // Ignores everything until the end of line token.
    ErrorRecovery,
}

#[derive(Debug)]
enum IntermediateState {
    Word(char),
    #[cfg(feature = "parse-parameters")]
    ParameterGet,
    #[cfg(feature = "parse-parameters")]
    ParameterSetId,
    #[cfg(feature = "parse-parameters")]
    ParameterSetValue(RealValue),
    #[cfg(feature = "parse-expressions")]
    Expression(Vec<Either<RealValue, Token>>),
    #[cfg(feature = "parse-expressions")]
    Unary(UnaryOperator),
    #[cfg(feature = "parse-expressions")]
    ATanComboFirst,
    #[cfg(feature = "parse-expressions")]
    ATanComboSecond(RealValue),
}

pub struct Parser<T> {
    lexer: lexer::Lexer<T>,
    state: State,
    stack: Stack<IntermediateState>,
    look_ahead: Option<Token>,
}

impl<T> Parser<T> {
    pub fn new(input: T) -> Self {
        Self {
            lexer: lexer::Lexer::new(input),
            state: State::StartOfLine,
            stack: Stack::new(),
            look_ahead: None,
        }
    }
}

impl<T> Parser<T> {
    fn unexpected_token<E: From<Error>>(&mut self, tok: Token) -> Option<Result<GCode, E>> {
        let state = mem::replace(&mut self.state, State::ErrorRecovery);
        self.stack.clear();
        Some(Err(Error::UnexpectedToken(state, tok).into()))
    }

    #[cfg_attr(not(feature = "parse-parameters"), allow(unused_mut))]
    fn pop_state<E>(&mut self, mut res: RealValue) -> Option<Result<GCode, E>> {
        loop {
            match self.stack.pop() {
                Some(IntermediateState::Word(c)) => {
                    self.state = State::Segments;
                    return Some(Ok(GCode::Word(c, res)));
                }
                #[cfg(feature = "parse-parameters")]
                Some(IntermediateState::ParameterGet) => {
                    res = RealValue::ParameterGet(Box::new(res))
                }
                #[cfg(feature = "parse-parameters")]
                Some(IntermediateState::ParameterSetId) => {
                    self.state = State::ParameterSet(res);
                    return None;
                }
                #[cfg(feature = "parse-parameters")]
                Some(IntermediateState::ParameterSetValue(id)) => {
                    self.state = State::Segments;
                    return Some(Ok(GCode::ParameterSet(id, res)));
                }
                #[cfg(feature = "parse-expressions")]
                Some(IntermediateState::Expression(mut ex)) => {
                    ex.push(Either::Left(res));
                    self.state = State::Expression(ex);
                    return None;
                }
                #[cfg(feature = "parse-expressions")]
                Some(IntermediateState::Unary(unop)) => {
                    self.state = State::UnaryExpressionEnd(unop, res);
                    return None;
                }
                #[cfg(feature = "parse-expressions")]
                Some(IntermediateState::ATanComboFirst) => {
                    self.state = State::ATanComboEnd(res);
                    return None;
                }
                #[cfg(feature = "parse-expressions")]
                Some(IntermediateState::ATanComboSecond(atan)) => {
                    self.state = State::ATanComboDividedByEnd(atan, res);
                    return None;
                }
                None => unreachable!(),
            }
        }
    }

    fn process_line<E: From<Error>>(&mut self, tok: Token) -> Option<Result<GCode, E>> {
        loop {
            match self.state {
                /* StartOfLine =>
                 *      Slash => emit BlockDelete, state = LineNumberOrSegment
                 *      EoL => ignore
                 *      t => look_ahead = t, state = LineNumberOrSegment
                 */
                State::StartOfLine => match &tok {
                    Token::Slash => return Some(Ok(GCode::BlockDelete)),
                    Token::EndOfLine => break,
                    _ => {
                        self.state = State::LineNumberOrSegments;
                    }
                },
                /* LineNumberOrSegment
                 *      Char('N') => state = LineNumber
                 *      EoL => emit Execute, state = StartOfLine
                 *      t => look_ahead = t, state = Segment
                 */
                State::LineNumberOrSegments => {
                    if tok == Token::Char('n') {
                        self.state = State::LineNumber;
                        break;
                    } else {
                        self.state = State::Segments;
                    }
                }
                /* LineNumber
                 *      Number => emit LineNumber(n.value), state = Segments
                 *      _ => unexpected token
                 */
                State::LineNumber => {
                    return if let Token::Number { value: v, .. } = tok {
                        self.state = State::Segments;
                        Some(Ok(GCode::LineNumber(v)))
                    } else {
                        self.unexpected_token(tok)
                    }
                }
                /* Segment
                 *      Comment => ignore
                 *      ParameterSign => state = ParamSet_Id
                 *      Char(c) => state = Word(c)
                 *      EoL => emit Execute, state = StartOfLine
                 *      _ => unexpected token
                 */
                State::Segments => match tok {
                    #[cfg(not(feature = "parse-comments"))]
                    Token::Comment => break,
                    #[cfg(feature = "parse-comments")]
                    Token::Comment(s) => return Some(Ok(GCode::Comment(s))),
                    #[cfg(feature = "parse-parameters")]
                    Token::ParameterSign => {
                        self.stack.push(IntermediateState::ParameterSetId);
                        self.state = State::RealValue(true);
                        break;
                    }
                    Token::Char(c) => {
                        self.stack.push(IntermediateState::Word(c));
                        self.state = State::RealValue(false);
                        break;
                    }
                    Token::EndOfLine => {
                        self.state = State::StartOfLine;
                        return Some(Ok(GCode::Execute));
                    }
                    tok => return self.unexpected_token(tok),
                },
                /* RealValue
                 *      RealNumber => [Sign] ((Number [dot [number])| (dot Number))
                 *      LeftBracket => // expect an expression
                 *      ParameterSign => // expect a parameter value
                 *      String => // expect a unary combo
                 *      _ => reset stack, unexpected token
                 */
                State::RealValue(_mandatory) => {
                    match tok {
                        Token::Plus => {
                            self.state = State::RealNumber(RealNumber::IntegerOrDot(false))
                        }
                        Token::Minus => {
                            self.state = State::RealNumber(RealNumber::IntegerOrDot(true))
                        }
                        Token::Number { value: v, .. } => {
                            self.state = State::RealNumber(RealNumber::Dot {
                                negative: false,
                                integer: v,
                                optional: true,
                            })
                        }
                        Token::Dot => {
                            self.state = State::RealNumber(RealNumber::Decimal {
                                negative: false,
                                integer: 0,
                                optional: false,
                            })
                        }
                        #[cfg(feature = "parse-parameters")]
                        Token::ParameterSign => {
                            self.stack.push(IntermediateState::ParameterGet);
                            self.state = State::RealValue(true);
                        }
                        #[cfg(feature = "parse-expressions")]
                        Token::LeftBracket => {
                            self.stack.push(IntermediateState::Expression(Vec::new()));
                            self.state = State::RealValue(true);
                        }
                        #[cfg(feature = "parse-expressions")]
                        Token::Operator(op) => {
                            if let Operator::ATan = op {
                                self.state = State::ATanComboStart;
                            } else {
                                match UnaryOperator::try_from(op) {
                                    Ok(unop) => {
                                        self.state = State::UnaryExpressionStart(unop);
                                    }
                                    Err(op) => return self.unexpected_token(Token::Operator(op)),
                                }
                            }
                            break;
                        }
                        #[cfg(not(feature = "optional-value"))]
                        tok => return self.unexpected_token(tok),
                        #[cfg(feature = "optional-value")]
                        tok => {
                            if _mandatory {
                                return self.unexpected_token(tok);
                            } else {
                                self.look_ahead = Some(tok);
                                return self.pop_state(RealValue::None);
                            }
                        }
                    }
                    break;
                }
                State::RealNumber(state) => {
                    // [sign] ( Number [ Dot [ Number ] ] | Dot Number)
                    match state {
                        RealNumber::IntegerOrDot(negative) => {
                            match tok {
                                Token::Number { value: integer, .. } => {
                                    self.state = State::RealNumber(RealNumber::Dot {
                                        negative,
                                        integer,
                                        optional: true,
                                    })
                                }
                                Token::Dot => {
                                    self.state = State::RealNumber(RealNumber::Decimal {
                                        negative,
                                        integer: 0,
                                        optional: false,
                                    })
                                }
                                tok => return self.unexpected_token(tok),
                            }
                            break;
                        }
                        RealNumber::Dot {
                            negative,
                            integer,
                            optional,
                        } => {
                            if let Token::Dot = tok {
                                self.state = State::RealNumber(RealNumber::Decimal {
                                    negative,
                                    integer,
                                    optional,
                                });
                                break;
                            } else if optional {
                                self.look_ahead = Some(tok);
                                let n = RealValue::build_real_number(negative, integer, 0, 10);
                                return self.pop_state(n);
                            } else {
                                return self.unexpected_token(tok);
                            }
                        }
                        RealNumber::Decimal {
                            negative,
                            integer,
                            optional,
                        } => {
                            return if let Token::Number {
                                value: dec,
                                factor: decfactor,
                            } = tok
                            {
                                let n =
                                    RealValue::build_real_number(negative, integer, dec, decfactor);
                                self.pop_state(n)
                            } else if optional {
                                self.look_ahead = Some(tok);
                                let n = RealValue::build_real_number(negative, integer, 0, 10);
                                self.pop_state(n)
                            } else {
                                self.unexpected_token(tok)
                            }
                        }
                    }
                }
                #[cfg(feature = "parse-parameters")]
                State::ParameterSet(ref mut id) => {
                    let id = mem::replace(id, RealValue::default());
                    self.stack.push(IntermediateState::ParameterSetValue(id));
                    self.state = State::RealValue(true);
                    break;
                }
                #[cfg(feature = "parse-expressions")]
                State::Expression(ref mut vec) => {
                    let mut vec = mem::replace(vec, Vec::new());
                    match tok {
                        Token::RightBracket => {
                            // build tree out of vec
                            match TryFrom::try_from(vec) {
                                Ok(exp) => {
                                    return self.pop_state(RealValue::Expression(Box::new(exp)))
                                }
                                _ => {
                                    unimplemented!();
                                }
                            }
                        }
                        Token::Plus | Token::Minus | Token::Times | Token::Slash | Token::Power => {
                            vec.push(Either::Right(tok));
                            self.stack.push(IntermediateState::Expression(vec));
                            self.state = State::RealValue(true);
                            break;
                        }
                        _ => {
                            return self.unexpected_token(tok);
                        }
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::UnaryExpressionStart(unop) => {
                    if let Token::LeftBracket = tok {
                        self.stack.push(IntermediateState::Unary(unop));
                        self.state = State::RealValue(true);
                        break;
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::UnaryExpressionEnd(unop, ref mut operand) => {
                    let operand = mem::replace(operand, RealValue::default());
                    if let Token::RightBracket = tok {
                        let expression = match unop {
                            UnaryOperator::Abs => Expression::Abs(operand),
                            UnaryOperator::ACos => Expression::ACos(operand),
                            UnaryOperator::ASin => Expression::ASin(operand),
                            UnaryOperator::Cos => Expression::Cos(operand),
                            UnaryOperator::Fix => Expression::Fix(operand),
                            UnaryOperator::Fup => Expression::Fup(operand),
                            UnaryOperator::Ln => Expression::Ln(operand),
                            UnaryOperator::Round => Expression::Round(operand),
                            UnaryOperator::Sin => Expression::Sin(operand),
                            UnaryOperator::Sqrt => Expression::Sqrt(operand),
                            UnaryOperator::Tan => Expression::Tan(operand),
                        };
                        return self.pop_state(RealValue::Expression(Box::new(expression)));
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::ATanComboStart => {
                    if let Token::LeftBracket = tok {
                        self.stack.push(IntermediateState::ATanComboFirst);
                        self.state = State::RealValue(true);
                        break;
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::ATanComboEnd(ref mut atan) => {
                    let atan = mem::replace(atan, RealValue::default());
                    if let Token::RightBracket = tok {
                        self.state = State::ATanComboDividedBy(atan);
                        break;
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::ATanComboDividedBy(ref mut atan) => {
                    let atan = mem::replace(atan, RealValue::default());
                    if let Token::Slash = tok {
                        self.state = State::ATanComboDividedByStart(atan);
                        break;
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::ATanComboDividedByStart(ref mut atan) => {
                    let atan = mem::replace(atan, RealValue::default());
                    if let Token::LeftBracket = tok {
                        self.stack.push(IntermediateState::ATanComboSecond(atan));
                        self.state = State::RealValue(true);
                        break;
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                #[cfg(feature = "parse-expressions")]
                State::ATanComboDividedByEnd(ref mut atan, ref mut divisor) => {
                    let atan = mem::replace(atan, RealValue::default());
                    let divisor = mem::replace(divisor, RealValue::default());
                    if let Token::RightBracket = tok {
                        return self.pop_state(RealValue::Expression(Box::new(Expression::ATan(
                            atan, divisor,
                        ))));
                    } else {
                        return self.unexpected_token(tok);
                    }
                }
                State::ErrorRecovery => {
                    if tok == Token::EndOfLine {
                        self.state = State::StartOfLine;
                        return Some(Ok(GCode::Execute));
                    }
                    break;
                }
            }
        }
        None
    }
}

impl<T, E> Iterator for Parser<T>
where
    T: Iterator<Item = Result<char, E>>,
    E: From<Error>,
{
    type Item = Result<GCode, E>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let tok = match self.look_ahead.take().map(Ok).or_else(|| self.lexer.next()) {
                Some(Ok(t)) => t,
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            };

            let v = self.process_line(tok);
            if v.is_some() {
                return v;
            }
        }
        #[allow(unreachable_code)]
        {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::{lexer::State as LexerState, Error, GCode, Parser, RealValue, State, Token};

    #[test]
    fn empty_lines_are_ignored() {
        let input = "\n\r\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(Parser::new(input).collect::<Vec<_>>(), &[]);
    }

    #[test]
    fn block_delete_emited_immediately() {
        let input = "/".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Ok(GCode::BlockDelete)]
        );
    }

    #[test]
    fn line_number() {
        let input = "N23\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Ok(GCode::LineNumber(23)), Ok(GCode::Execute)]
        );

        let input = "N0023\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Ok(GCode::LineNumber(23)), Ok(GCode::Execute)]
        );
    }

    #[test]
    fn incomplete_line_number() {
        let input = "N\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Err(Error::UnexpectedToken(
                State::LineNumber,
                Token::EndOfLine
            ))]
        );
    }

    #[test]
    #[cfg(not(feature = "parse-comments"))]
    fn well_formed_comments_are_ignored() {
        let input = "(Hello world ! This is a comment.)"
            .chars()
            .map(Result::<char, Error>::Ok);

        assert_eq!(Parser::new(input).collect::<Vec<_>>(), &[]);
    }

    #[test]
    #[cfg(feature = "parse-comments")]
    fn well_formed_comments_are_emited() {
        let input = "(Hello world ! This is a comment.)"
            .chars()
            .map(Result::<char, Error>::Ok);

        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Ok(GCode::Comment(
                "hello world ! this is a comment.".to_string()
            ))]
        );
    }

    #[test]
    fn open_parenthesis_are_not_allowed_inside_comments() {
        let input = "(Hello world ! (".chars().map(Result::<char, Error>::Ok);

        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Err(Error::UnexpectedChar(LexerState::Idle, '(')),]
        );

        let input = "(Hello world !\n".chars().map(Result::<char, Error>::Ok);

        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[Err(Error::UnexpectedChar(LexerState::Idle, '\n')),]
        );
    }

    #[test]
    fn word_with_number() {
        let input = "G21H21.I21.098J-21K-21.L-21.098M+21N+21.P+21.098Q.098R-.098S+.098\n"
            .chars()
            .map(Result::<char, Error>::Ok);

        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[
                Ok(GCode::Word(
                    'g',
                    RealValue::build_real_number(false, 21, 0, 10)
                )),
                Ok(GCode::Word(
                    'h',
                    RealValue::build_real_number(false, 21, 0, 10)
                )),
                Ok(GCode::Word(
                    'i',
                    RealValue::build_real_number(false, 21, 98, 1000)
                )),
                Ok(GCode::Word(
                    'j',
                    RealValue::build_real_number(true, 21, 0, 10)
                )),
                Ok(GCode::Word(
                    'k',
                    RealValue::build_real_number(true, 21, 0, 10)
                )),
                Ok(GCode::Word(
                    'l',
                    RealValue::build_real_number(true, 21, 98, 1000)
                )),
                Ok(GCode::Word(
                    'm',
                    RealValue::build_real_number(false, 21, 0, 10)
                )),
                Ok(GCode::Word(
                    'n',
                    RealValue::build_real_number(false, 21, 0, 10)
                )),
                Ok(GCode::Word(
                    'p',
                    RealValue::build_real_number(false, 21, 98, 1000)
                )),
                Ok(GCode::Word(
                    'q',
                    RealValue::build_real_number(false, 0, 98, 1000)
                )),
                Ok(GCode::Word(
                    'r',
                    RealValue::build_real_number(true, 0, 98, 1000)
                )),
                Ok(GCode::Word(
                    's',
                    RealValue::build_real_number(false, 0, 98, 1000)
                )),
                Ok(GCode::Execute)
            ]
        );
    }

    #[test]
    #[cfg(feature = "parse-parameters")]
    fn parse_param_get() {
        let input = "G#-21.098\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[
                Ok(GCode::Word(
                    'g',
                    RealValue::ParameterGet(Box::new(RealValue::build_real_number(
                        true, 21, 98, 1000
                    )))
                )),
                Ok(GCode::Execute)
            ]
        )
    }

    #[test]
    #[cfg(feature = "parse-parameters")]
    fn parse_param_set() {
        let input = "#23.4=#-75.8\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[
                Ok(GCode::ParameterSet(
                    RealValue::build_real_number(false, 23, 4, 10),
                    RealValue::ParameterGet(Box::new(RealValue::build_real_number(
                        true, 75, 8, 10
                    )))
                )),
                Ok(GCode::Execute)
            ]
        )
    }

    #[test]
    #[cfg(feature = "parse-expressions")]
    fn parse_expressions() {
        use crate::expressions::Expression;
        let (sender, receiver) = std::sync::mpsc::sync_channel(128);
        let parser = &mut Parser::new(receiver.try_iter());

        "G[3+2]\n".chars().for_each(|c| {
            sender
                .send(Result::<char, Error>::Ok(c))
                .expect("Send failure")
        });
        assert_eq!(
            parser.collect::<Vec<_>>(),
            &[
                Ok(GCode::Word(
                    'g',
                    RealValue::Expression(Box::new(Expression::Add(
                        RealValue::build_real_number(false, 3, 0, 10),
                        RealValue::build_real_number(false, 2, 0, 10)
                    )))
                )),
                Ok(GCode::Execute)
            ]
        );
        "GCos[90] GaTan[2.3]/[28]\n".chars().for_each(|c| {
            sender
                .send(Result::<char, Error>::Ok(c))
                .expect("Send failure")
        });
        assert_eq!(
            parser.collect::<Vec<_>>(),
            &[
                Ok(GCode::Word(
                    'g',
                    RealValue::Expression(Box::new(Expression::Cos(RealValue::build_real_number(
                        false, 90, 0, 10
                    ),)))
                )),
                Ok(GCode::Word(
                    'g',
                    RealValue::Expression(Box::new(Expression::ATan(
                        RealValue::build_real_number(false, 2, 3, 10),
                        RealValue::build_real_number(false, 28, 0, 10)
                    )))
                )),
                Ok(GCode::Execute)
            ]
        );
        #[cfg(feature = "parse-parameters")]
        {
            "G[3.--5**[#2.5*2-2]+Cos[#2]/3]\n".chars().for_each(|c| {
                sender
                    .send(Result::<char, Error>::Ok(c))
                    .expect("Send failure")
            });
            assert_eq!(
                parser.collect::<Vec<_>>(),
                &[
                    Ok(GCode::Word(
                        'g',
                        RealValue::Expression(Box::new(Expression::Add(
                            RealValue::Expression(Box::new(Expression::Sub(
                                RealValue::build_real_number(false, 3, 0, 10),
                                RealValue::Expression(Box::new(Expression::Pow(
                                    RealValue::build_real_number(true, 5, 0, 10),
                                    RealValue::Expression(Box::new(Expression::Sub(
                                        RealValue::Expression(Box::new(Expression::Mul(
                                            RealValue::ParameterGet(Box::new(
                                                RealValue::build_real_number(false, 2, 5, 10)
                                            )),
                                            RealValue::build_real_number(false, 2, 0, 10)
                                        ))),
                                        RealValue::build_real_number(false, 2, 0, 10)
                                    )))
                                )))
                            ))),
                            RealValue::Expression(Box::new(Expression::Div(
                                RealValue::Expression(Box::new(Expression::Cos(
                                    RealValue::ParameterGet(Box::new(
                                        RealValue::build_real_number(false, 2, 0, 10)
                                    ))
                                ))),
                                RealValue::build_real_number(false, 3, 0, 10)
                            )))
                        )))
                    )),
                    Ok(GCode::Execute)
                ]
            )
        }
    }

    #[cfg(feature = "optional-value")]
    #[test]
    fn code_may_not_have_a_value() {
        let input = "G75 Z T48 S P.3\n".chars().map(Result::<char, Error>::Ok);
        assert_eq!(
            Parser::new(input).collect::<Vec<_>>(),
            &[
                Ok(GCode::Word(
                    'g',
                    RealValue::build_real_number(false, 75, 0, 10),
                )),
                Ok(GCode::Word('z', RealValue::None,)),
                Ok(GCode::Word(
                    't',
                    RealValue::build_real_number(false, 48, 0, 10),
                )),
                Ok(GCode::Word('s', RealValue::None,)),
                Ok(GCode::Word(
                    'p',
                    RealValue::build_real_number(false, 0, 3, 10),
                )),
                Ok(GCode::Execute)
            ]
        );
        #[cfg(feature = "parse-parameters")]
        {
            let input = "G#\n".chars().map(Result::<char, Error>::Ok);
            assert_eq!(
                Parser::new(input).collect::<Vec<_>>(),
                &[Err(Error::UnexpectedToken(
                    State::RealValue(true),
                    Token::EndOfLine
                ))]
            );
        }
    }
}
