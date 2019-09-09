use crate::std::mem;
#[cfg(feature = "parse-comments")]
use crate::std::string::String;
#[cfg(feature = "parse-expressions")]
use crate::utils::Either;
use crate::Error;

trait IsValidWordChar {
    fn is_valid_word_char(&self) -> bool;
}
#[cfg(feature = "extended")]
impl IsValidWordChar for char {
    fn is_valid_word_char(&self) -> bool {
        self.is_ascii_alphabetic()
    }
}
#[cfg(not(feature = "extended"))]
impl IsValidWordChar for char {
    fn is_valid_word_char(&self) -> bool {
        self.is_ascii_alphabetic() && !find_in_str("eouvw", *self)
    }
}

fn find_in_str(input: &str, needle: char) -> bool {
    input.chars().any(|a| a == needle)
}

#[cfg(feature = "parse-expressions")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Abs,
    And,
    ACos,
    ASin,
    ATan,
    Cos,
    Pow,
    Fix,
    Fup,
    Ln,
    Mod,
    Or,
    Round,
    Sin,
    Sqrt,
    Tan,
    Xor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Slash,
    Char(char),
    Plus,
    Minus,
    Number {
        value: u32,
        factor: u32,
    },
    Dot,
    EndOfLine,
    #[cfg(not(feature = "parse-comments"))]
    Comment,
    #[cfg(feature = "parse-comments")]
    Comment(String),
    #[cfg(feature = "parse-expressions")]
    Operator(Operator),
    #[cfg(feature = "parse-expressions")]
    LeftBracket,
    #[cfg(feature = "parse-expressions")]
    RightBracket,
    #[cfg(feature = "parse-expressions")]
    Times,
    #[cfg(feature = "parse-expressions")]
    Power,
    #[cfg(feature = "parse-parameters")]
    ParameterSign,
    #[cfg(feature = "parse-parameters")]
    Equal,
}

#[cfg(feature = "parse-expressions")]
#[derive(Debug, PartialEq)]
pub enum OperatorState {
    Abs,
    And,
    ACosExpectO,
    ACosExpectS,
    ASinExpectI,
    ASinExpectN,
    ATanExpectA,
    ATanExpectN,
    Cos,
    Pow,
    Fix,
    Fup,
    Mod,
    RoundExpectU,
    RoundExpectN,
    RoundExpectD,
    Sin,
    SqrtExpectR,
    SqrtExpectT,
    Tan,
    Xor,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Idle,
    #[cfg(feature = "parse-comments")]
    Comment(String),
    #[cfg(not(feature = "parse-comments"))]
    Comment,
    #[cfg(all(feature = "parse-comments", feature = "extended"))]
    SemiColonComment(String),
    #[cfg(all(not(feature = "parse-comments"), feature = "extended"))]
    SemiColonComment,
    #[cfg(feature = "parse-expressions")]
    TimesOrPower,
    #[cfg(feature = "parse-expressions")]
    Operator(OperatorState),
    Number(u32, u32),
    ErrorRecovery,
}

pub struct Lexer<T> {
    input: T,
    state: State,
    look_ahead: Option<char>,
}

impl<T> Lexer<T> {
    pub fn new(input: T) -> Self {
        Self {
            input,
            state: State::Idle,
            look_ahead: None,
        }
    }
}
impl<T, E> Lexer<T>
where
    T: Iterator<Item = Result<char, E>>,
    E: From<Error>,
{
    fn unexpected_char(&mut self, c: char) -> Result<Token, E> {
        let s = mem::replace(&mut self.state, State::ErrorRecovery);
        Err(Error::UnexpectedChar(s, c).into())
    }
}
impl<T, E> Iterator for Lexer<T>
where
    T: Iterator<Item = Result<char, E>>,
    E: From<Error>,
{
    type Item = Result<Token, E>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut c = match self
            .look_ahead
            .take()
            .map(Ok)
            .or_else(|| self.input.next())
        {
            Some(Ok(c)) => c.to_ascii_lowercase(),
            Some(Err(e)) => return Some(Err(e)),
            None => return None,
        };
        loop {
            match self.state {
                State::Idle => match c {
                    '\t' | ' ' => {}
                    #[cfg(feature = "parse-comments")]
                    '(' => self.state = State::Comment(String::new()),
                    #[cfg(not(feature = "parse-comments"))]
                    '(' => self.state = State::Comment,
                    #[cfg(all(feature = "parse-comments", feature = "extended"))]
                    ';' => self.state = State::SemiColonComment(String::new()),
                    #[cfg(all(not(feature = "parse-comments"), feature = "extended"))]
                    ';' => self.state = State::SemiColonComment,
                    #[cfg(feature = "parse-expressions")]
                    '*' => self.state = State::TimesOrPower,
                    c => {
                        if let Some(v) = c.to_digit(10) {
                            self.state = State::Number(v, 10)
                        } else if c.is_ascii_alphabetic() {
                            #[cfg(feature = "parse-expressions")]
                            match self.input.next() {
                                Some(Err(e)) => {
                                    self.look_ahead = Some(c);
                                    return Some(Err(e));
                                }
                                None => {
                                    self.look_ahead = Some(c);
                                    return None;
                                }
                                Some(Ok(d)) => {
                                    self.state =
                                        State::Operator(match (c, d.to_ascii_lowercase()) {
                                            // Three or more chars unaries
                                            ('a', 'b') => OperatorState::Abs,
                                            ('a', 'n') => OperatorState::And,
                                            ('a', 'c') => OperatorState::ACosExpectO,
                                            ('a', 's') => OperatorState::ASinExpectI,
                                            ('a', 't') => OperatorState::ATanExpectA,
                                            ('c', 'o') => OperatorState::Cos,
                                            ('e', 'x') => OperatorState::Pow,
                                            ('f', 'i') => OperatorState::Fix,
                                            ('f', 'u') => OperatorState::Fup,
                                            ('r', 'o') => OperatorState::RoundExpectU,
                                            ('s', 'i') => OperatorState::Sin,
                                            ('s', 'q') => OperatorState::SqrtExpectR,
                                            ('t', 'a') => OperatorState::Tan,
                                            ('x', 'o') => OperatorState::Xor,
                                            ('m', 'o') => OperatorState::Mod,
                                            // Two chars unaries
                                            ('l', 'n') => {
                                                self.state = State::Idle;
                                                return Some(Ok(Token::Operator(Operator::Ln)));
                                            }
                                            ('o', 'r') => {
                                                self.state = State::Idle;
                                                return Some(Ok(Token::Operator(Operator::Or)));
                                            }
                                            // default
                                            (c, d) => {
                                                self.look_ahead = Some(d);
                                                return if c.is_valid_word_char() {
                                                    Some(Ok(Token::Char(c)))
                                                } else {
                                                    Some(self.unexpected_char(c))
                                                };
                                            }
                                        });
                                }
                            };

                            #[cfg(not(feature = "parse-expressions"))]
                            return if c.is_valid_word_char() {
                                Some(Ok(Token::Char(c)))
                            } else {
                                Some(self.unexpected_char(c))
                            };
                        } else {
                            return Some(match c {
                                '/' => Ok(Token::Slash),
                                '.' => Ok(Token::Dot),
                                '+' => Ok(Token::Plus),
                                '-' => Ok(Token::Minus),
                                '\r' | '\n' => Ok(Token::EndOfLine),
                                #[cfg(feature = "parse-expressions")]
                                '[' => Ok(Token::LeftBracket),
                                #[cfg(feature = "parse-expressions")]
                                ']' => Ok(Token::RightBracket),
                                #[cfg(feature = "parse-parameters")]
                                '#' => Ok(Token::ParameterSign),
                                #[cfg(feature = "parse-parameters")]
                                '=' => Ok(Token::Equal),
                                c => self.unexpected_char(c),
                            });
                        }
                    }
                },
                State::Number(ref mut n, ref mut factor) => match c.to_digit(10) {
                    Some(v) => {
                        *n = *n * 10 + v;
                        *factor *= 10;
                    }
                    None => {
                        let (value, factor) = (*n, *factor);
                        self.look_ahead = Some(c);
                        self.state = State::Idle;
                        return Some(Ok(Token::Number {
                            value,
                            factor,
                        }));
                    }
                },
                #[cfg(feature = "parse-expressions")]
                State::TimesOrPower => {
                    self.state = State::Idle;
                    return if c == '*' {
                        Some(Ok(Token::Power))
                    } else {
                        self.look_ahead = Some(c);
                        Some(Ok(Token::Times))
                    };
                }
                #[cfg(feature = "parse-expressions")]
                State::Operator(ref mut us) => {
                    let out = match (&us, &c) {
                        (OperatorState::Abs, 's') => Either::Left(Operator::Abs),
                        (OperatorState::And, 'd') => Either::Left(Operator::And),
                        (OperatorState::ACosExpectO, 'o') => {
                            Either::Right(OperatorState::ACosExpectS)
                        }
                        (OperatorState::ACosExpectS, 's') => Either::Left(Operator::ACos),
                        (OperatorState::ASinExpectI, 'i') => {
                            Either::Right(OperatorState::ASinExpectN)
                        }
                        (OperatorState::ASinExpectN, 'n') => Either::Left(Operator::ASin),
                        (OperatorState::ATanExpectA, 'a') => {
                            Either::Right(OperatorState::ATanExpectN)
                        }
                        (OperatorState::ATanExpectN, 'n') => Either::Left(Operator::ATan),
                        (OperatorState::Cos, 's') => Either::Left(Operator::Cos),
                        (OperatorState::Pow, 'p') => Either::Left(Operator::Pow),
                        (OperatorState::Fix, 'x') => Either::Left(Operator::Fix),
                        (OperatorState::Fup, 'p') => Either::Left(Operator::Fup),
                        (OperatorState::Mod, 'd') => Either::Left(Operator::Mod),
                        (OperatorState::RoundExpectU, 'u') => {
                            Either::Right(OperatorState::RoundExpectN)
                        }
                        (OperatorState::RoundExpectN, 'n') => {
                            Either::Right(OperatorState::RoundExpectD)
                        }
                        (OperatorState::RoundExpectD, 'd') => Either::Left(Operator::Round),
                        (OperatorState::Sin, 'n') => Either::Left(Operator::Sin),
                        (OperatorState::SqrtExpectR, 'r') => {
                            Either::Right(OperatorState::SqrtExpectT)
                        }
                        (OperatorState::SqrtExpectT, 't') => Either::Left(Operator::Sqrt),
                        (OperatorState::Tan, 'n') => Either::Left(Operator::Tan),
                        (OperatorState::Xor, 'r') => Either::Left(Operator::Xor),
                        (_, _) => return Some(self.unexpected_char(c)),
                    };
                    match out {
                        Either::Left(left) => {
                            self.state = State::Idle;
                            return Some(Ok(Token::Operator(left)));
                        }
                        Either::Right(right) => *us = right,
                    }
                }
                #[cfg(not(feature = "parse-comments"))]
                State::Comment => {
                    match c {
                        ')' => {
                            self.state = State::Idle;
                            return Some(Ok(Token::Comment));
                        }
                        c if find_in_str("(\r\n", c) => {
                            self.state = State::Idle;
                            return Some(self.unexpected_char(c));
                        }
                        _ => {}
                    };
                }
                #[cfg(feature = "parse-comments")]
                State::Comment(ref mut s) => {
                    match c {
                        ')' => {
                            let msg = mem::replace(s, String::new());
                            self.state = State::Idle;
                            return Some(Ok(Token::Comment(msg)));
                        }
                        c if !find_in_str("(\r\n", c) => {
                            s.push(c);
                        }
                        c => {
                            self.state = State::Idle;
                            return Some(self.unexpected_char(c));
                        }
                    };
                }
                #[cfg(all(not(feature = "parse-comments"), feature = "extended"))]
                State::SemiColonComment => {
                    if c == '\r' || c == '\n' {
                        self.state = State::Idle;
                        self.look_ahead = Some(c);
                        return Some(Ok(Token::Comment));
                    }
                }
                #[cfg(all(feature = "parse-comments", feature = "extended"))]
                State::SemiColonComment(ref mut s) => {
                    if c == '\r' || c == '\n' {
                        let msg = mem::replace(s, String::new());
                        self.state = State::Idle;
                        self.look_ahead = Some(c);
                        return Some(Ok(Token::Comment(msg)));
                    } else {
                        s.push(c)
                    }
                }
                State::ErrorRecovery => {
                    if c == '\r' || c == '\n' {
                        self.state = State::Idle;
                        return Some(Ok(Token::EndOfLine));
                    }
                }
            };
            c = match self.input.next() {
                Some(Ok(c)) => c.to_ascii_lowercase(),
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            };
        }
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "parse-expressions")]
    use super::Operator;
    use super::{Error, Lexer, Token};

    #[test]
    fn parse_all() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(128);
        let lexer = &mut Lexer::new(receiver.try_iter());

        "\r + - ( this is a comment ) . 32 A\n"
            .chars()
            .for_each(|c| {
                sender
                    .send(Result::<char, Error>::Ok(c))
                    .expect("Send failure")
            });
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            &[
                Ok(Token::EndOfLine),
                Ok(Token::Plus),
                Ok(Token::Minus),
                #[cfg(feature = "parse-comments")]
                Ok(Token::Comment(" this is a comment ".to_string())),
                #[cfg(not(feature = "parse-comments"))]
                Ok(Token::Comment),
                Ok(Token::Dot),
                Ok(Token::Number {
                    value: 32,
                    factor: 100,
                }),
                Ok(Token::Char('a')),
                Ok(Token::EndOfLine),
            ]
        );
        #[cfg(feature = "parse-expressions")]
        {
            "[ ] / * ** and".chars().for_each(|c| {
                sender
                    .send(Result::<char, Error>::Ok(c))
                    .expect("Send failure")
            });
            assert_eq!(
                lexer.collect::<Vec<_>>(),
                &[
                    Ok(Token::LeftBracket),
                    Ok(Token::RightBracket),
                    Ok(Token::Slash),
                    Ok(Token::Times),
                    Ok(Token::Power),
                    Ok(Token::Operator(Operator::And)),
                ]
            );
        }
        #[cfg(feature = "parse-parameters")]
        {
            "# = \n".chars().for_each(|c| {
                sender
                    .send(Result::<char, Error>::Ok(c))
                    .expect("Send failure")
            });
            assert_eq!(
                lexer.collect::<Vec<_>>(),
                &[
                    Ok(Token::ParameterSign),
                    Ok(Token::Equal),
                    Ok(Token::EndOfLine)
                ]
            );
        }

        #[cfg(feature = "extended")]
        {
            "; This is another comment\n".chars().for_each(|c| {
                sender
                    .send(Result::<char, Error>::Ok(c))
                    .expect("Send failure")
            });
            assert_eq!(
                lexer.collect::<Vec<_>>(),
                &[
                    #[cfg(feature = "parse-comments")]
                    Ok(Token::Comment(" this is another comment".to_string())),
                    #[cfg(not(feature = "parse-comments"))]
                    Ok(Token::Comment),
                    Ok(Token::EndOfLine)
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "parse-expressions")]
    fn wait_for_next_char_before_emitting_tok_char() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(10);
        let lexer = &mut Lexer::new(receiver.try_iter());

        sender
            .send(Result::<char, Error>::Ok('a'))
            .expect("Send failure");

        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, &[]);

        sender
            .send(Result::<char, Error>::Ok('\n'))
            .expect("Send failure");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, &[Ok(Token::Char('a')), Ok(Token::EndOfLine)]);
    }

    #[test]
    fn partial_reception() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(10);
        let lexer = &mut Lexer::new(receiver.try_iter());

        "Z2".chars().for_each(|c| {
            sender
                .send(Result::<char, Error>::Ok(c))
                .expect("Send failure")
        });

        assert_eq!(lexer.collect::<Vec<_>>(), &[Ok(Token::Char('z'))]);

        "3\n".chars().for_each(|c| {
            sender
                .send(Result::<char, Error>::Ok(c))
                .expect("Send failure");
        });

        assert_eq!(
            lexer.collect::<Vec<_>>(),
            &[
                Ok(Token::Number {
                    value: 23,
                    factor: 100
                }),
                Ok(Token::EndOfLine)
            ]
        );
    }

    #[cfg(not(feature = "extended"))]
    #[test]
    fn eouvw_are_illegal_chars() {
        use super::State;
        let tokens: Vec<_> = Lexer::new(
            "E\ne\nO\no\nU\nu\nV\nv\nW\nw\n"
                .chars()
                .map(|v| Result::<char, Error>::Ok(v)),
        )
        .collect();
        assert_eq!(
            tokens,
            &[
                Err(Error::UnexpectedChar(State::Idle, 'e')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'e')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'o')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'o')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'u')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'u')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'v')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'v')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'w')),
                Ok(Token::EndOfLine),
                Err(Error::UnexpectedChar(State::Idle, 'w')),
                Ok(Token::EndOfLine),
            ]
        );
    }

    #[test]
    #[cfg(feature = "parse-expressions")]
    fn parse_unary() {
        let unary_combos = "absandacosasinatancosxorexpfixfupmodlnorroundsinsqrttan";
        let tokens: Vec<_> =
            Lexer::new(unary_combos.chars().map(|v| Result::<char, Error>::Ok(v))).collect();
        assert_eq!(
            tokens,
            &[
                Ok(Token::Operator(Operator::Abs)),
                Ok(Token::Operator(Operator::And)),
                Ok(Token::Operator(Operator::ACos)),
                Ok(Token::Operator(Operator::ASin)),
                Ok(Token::Operator(Operator::ATan)),
                Ok(Token::Operator(Operator::Cos)),
                Ok(Token::Operator(Operator::Xor)),
                Ok(Token::Operator(Operator::Pow)),
                Ok(Token::Operator(Operator::Fix)),
                Ok(Token::Operator(Operator::Fup)),
                Ok(Token::Operator(Operator::Mod)),
                Ok(Token::Operator(Operator::Ln)),
                Ok(Token::Operator(Operator::Or)),
                Ok(Token::Operator(Operator::Round)),
                Ok(Token::Operator(Operator::Sin)),
                Ok(Token::Operator(Operator::Sqrt)),
                Ok(Token::Operator(Operator::Tan)),
            ]
        );
    }

    #[test]
    fn nowhitespaces() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(128);
        let lexer = &mut Lexer::new(receiver.try_iter());

        "N0022b23\n".chars().for_each(|c| {
            sender
                .send(Result::<char, Error>::Ok(c))
                .expect("Send failure")
        });
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            &[
                Ok(Token::Char('n')),
                Ok(Token::Number {
                    value: 22,
                    factor: 10000
                }),
                Ok(Token::Char('b')),
                Ok(Token::Number {
                    value: 23,
                    factor: 100
                }),
                Ok(Token::EndOfLine)
            ]
        );

        #[cfg(feature = "parse-expressions")]
        {
            "aAtAn[76]/[5]".chars().for_each(|c| {
                sender
                    .send(Result::<char, Error>::Ok(c))
                    .expect("Send failure")
            });
            assert_eq!(
                lexer.collect::<Vec<_>>(),
                &[
                    Ok(Token::Char('a')),
                    Ok(Token::Operator(Operator::ATan)),
                    Ok(Token::LeftBracket),
                    Ok(Token::Number {
                        value: 76,
                        factor: 100
                    }),
                    Ok(Token::RightBracket),
                    Ok(Token::Slash),
                    Ok(Token::LeftBracket),
                    Ok(Token::Number {
                        value: 5,
                        factor: 10
                    }),
                    Ok(Token::RightBracket),
                ]
            );
        }
    }
}
