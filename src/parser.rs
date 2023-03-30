use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use core::ops::Range;

use crate::tokenizer::{self, Balanced, Integer, Token, TokenKind, Tokenizer};

pub struct Parser<'s> {
    tokens: Tokenizer<'s, false>,
    peeked: Option<Result<Token<'s>, tokenizer::Error>>,
    nested: Vec<NestedState>,
    finished: bool,
}

impl<'s> Parser<'s> {
    pub fn new(source: &'s str) -> Self {
        Self {
            tokens: Tokenizer::minified(source),
            peeked: None,
            nested: Vec::new(),
            finished: false,
        }
    }

    fn peek(&mut self) -> Option<&Token<'s>> {
        if self.peeked.is_none() {
            self.peeked = self.tokens.next();
        }

        self.peeked.as_ref().and_then(|r| r.as_ref().ok())
    }

    fn next_token(&mut self) -> Option<Result<Token<'s>, tokenizer::Error>> {
        self.peeked.take().or_else(|| self.tokens.next())
    }

    fn next_or_eof(&mut self) -> Result<Token<'s>, Error> {
        match self.next_token() {
            Some(Ok(token)) => Ok(token),
            Some(Err(err)) => Err(err.into()),
            None => Err(Error::new(
                self.tokens.current_offset()..self.tokens.current_offset(),
                ErrorKind::UnexpectedEof,
            )),
        }
    }

    fn parse_token(
        &mut self,
        token: Token<'s>,
        allowed_close: Option<Balanced>,
    ) -> Result<Event<'s>, Error> {
        match token.kind {
            TokenKind::Integer(integer) => Ok(Event::Primitive(Primitive::Integer(integer))),
            TokenKind::Float(float) => Ok(Event::Primitive(Primitive::Float(float))),
            TokenKind::Bool(value) => Ok(Event::Primitive(Primitive::Bool(value))),
            TokenKind::Character(value) => Ok(Event::Primitive(Primitive::Char(value))),
            TokenKind::Byte(value) => Ok(Event::Primitive(Primitive::Integer(Integer::Usize(
                value as usize,
            )))),
            TokenKind::String(value) => Ok(Event::Primitive(Primitive::String(value))),
            TokenKind::Bytes(value) => Ok(Event::Primitive(Primitive::Bytes(value))),
            TokenKind::Identifier(value) => {
                if matches!(
                    self.peek(),
                    Some(Token {
                        kind: TokenKind::Open(Balanced::Brace | Balanced::Paren),
                        ..
                    })
                ) {
                    let Some(Ok(Token { kind: TokenKind::Open(balanced), .. })) = self.next_token() else { unreachable!("matched above") };

                    let kind = match balanced {
                        Balanced::Paren => {
                            self.nested
                                .push(NestedState::Tuple(ListState::ExpectingValue));
                            Nested::Tuple
                        }
                        Balanced::Brace => {
                            self.nested.push(NestedState::Map(MapState::ExpectingKey));
                            Nested::Map
                        }
                        Balanced::Bracket => {
                            unreachable!("specifically excluded above")
                        }
                    };

                    Ok(Event::BeginNested {
                        name: Some(value),
                        kind,
                    })
                } else {
                    Ok(Event::Primitive(Primitive::Identifier(value)))
                }
            }
            TokenKind::Open(Balanced::Paren) => {
                self.nested
                    .push(NestedState::Tuple(ListState::ExpectingValue));
                Ok(Event::BeginNested {
                    name: None,
                    kind: Nested::Tuple,
                })
            }
            TokenKind::Open(Balanced::Bracket) => {
                self.nested
                    .push(NestedState::List(ListState::ExpectingValue));
                Ok(Event::BeginNested {
                    name: None,
                    kind: Nested::List,
                })
            }
            TokenKind::Open(Balanced::Brace) => {
                self.nested.push(NestedState::Map(MapState::ExpectingKey));
                Ok(Event::BeginNested {
                    name: None,
                    kind: Nested::Map,
                })
            }
            TokenKind::Close(closed) if Some(closed) == allowed_close => {
                self.nested.pop();
                Ok(Event::EndNested)
            }
            TokenKind::Colon | TokenKind::Comma | TokenKind::Close(_) => {
                todo!("expected value, got something else.")
            }
            TokenKind::Comment(_) | TokenKind::Whitespace => unreachable!("disabled"),
        }
    }

    fn parse_sequence(&mut self, state: ListState, end: Balanced) -> Result<Event<'s>, Error> {
        match state {
            ListState::ExpectingValue => {
                *self.nested.last_mut().expect("required for this fn") =
                    NestedState::list(end, ListState::ExpectingComma);

                let token = self.next_or_eof()?;
                self.parse_token(token, Some(end))
            }
            ListState::ExpectingComma => {
                let token = self.next_or_eof()?;
                match token.kind {
                    TokenKind::Close(closed) if closed == end => {
                        self.nested.pop();
                        Ok(Event::EndNested)
                    }
                    TokenKind::Comma => {
                        *self.nested.last_mut().expect("required for this fn") =
                            NestedState::list(end, ListState::ExpectingValue);
                        self.parse_sequence(ListState::ExpectingValue, end)
                    }
                    _ => todo!("expected comma or end"),
                }
            }
        }
    }

    fn parse_map(&mut self, state: MapState) -> Result<Event<'s>, Error> {
        match state {
            MapState::ExpectingKey => {
                *self.nested.last_mut().expect("required for this fn") =
                    NestedState::Map(MapState::ExpectingColon);

                let token = self.next_or_eof()?;
                self.parse_token(token, Some(Balanced::Brace))
            }
            MapState::ExpectingColon => {
                let token = self.next_or_eof()?;
                if matches!(token.kind, TokenKind::Colon) {
                    *self.nested.last_mut().expect("required for this fn") =
                        NestedState::Map(MapState::ExpectingValue);
                    self.parse_map(MapState::ExpectingValue)
                } else {
                    todo!("expected colon, got {token:?}")
                }
            }
            MapState::ExpectingValue => {
                *self.nested.last_mut().expect("required for this fn") =
                    NestedState::Map(MapState::ExpectingComma);

                let token = self.next_or_eof()?;
                self.parse_token(token, Some(Balanced::Brace))
            }
            MapState::ExpectingComma => {
                let token = self.next_or_eof()?;
                match token.kind {
                    TokenKind::Close(closed) if closed == Balanced::Brace => {
                        self.nested.pop();
                        Ok(Event::EndNested)
                    }
                    TokenKind::Comma => {
                        *self.nested.last_mut().expect("required for this fn") =
                            NestedState::Map(MapState::ExpectingKey);
                        self.parse_map(MapState::ExpectingKey)
                    }
                    _ => todo!("expected comma or end"),
                }
            }
        }
    }
}

impl<'s> Iterator for Parser<'s> {
    type Item = Result<Event<'s>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.nested.last() {
            None => match self.next_token()? {
                Ok(token) => {
                    if self.finished {
                        todo!("error: trailing junk")
                    }

                    self.finished = true;
                    self.parse_token(token, None)
                }
                Err(err) => Err(err.into()),
            },
            Some(NestedState::Tuple(list)) => self.parse_sequence(*list, Balanced::Paren),
            Some(NestedState::List(list)) => self.parse_sequence(*list, Balanced::Bracket),
            Some(NestedState::Map(map)) => self.parse_map(*map),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    pub location: Range<usize>,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(location: Range<usize>, kind: ErrorKind) -> Self {
        Self { location, kind }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match &self.kind {
            ErrorKind::Tokenizer(err) => Display::fmt(err, f),
            ErrorKind::UnexpectedEof => f.write_str("unexpected end of file"),
        }
    }
}

impl From<tokenizer::Error> for Error {
    fn from(err: tokenizer::Error) -> Self {
        Self {
            location: err.location,
            kind: ErrorKind::Tokenizer(err.kind),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    Tokenizer(tokenizer::ErrorKind),
    UnexpectedEof,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Event<'s> {
    BeginNested {
        name: Option<Cow<'s, str>>,
        kind: Nested,
    },
    EndNested,
    Primitive(Primitive<'s>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Nested {
    Tuple,
    Map,
    List,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NestedState {
    Tuple(ListState),
    List(ListState),
    Map(MapState),
}

impl NestedState {
    fn list(kind: Balanced, state: ListState) -> Self {
        match kind {
            Balanced::Paren => Self::Tuple(state),
            Balanced::Bracket => Self::List(state),
            Balanced::Brace => unreachable!("Brace must receive a MapState"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ListState {
    ExpectingValue,
    ExpectingComma,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MapState {
    ExpectingKey,
    ExpectingColon,
    ExpectingValue,
    ExpectingComma,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Primitive<'s> {
    Bool(bool),
    Integer(Integer),
    Float(f64),
    Char(char),
    String(Cow<'s, str>),
    Identifier(Cow<'s, str>),
    Bytes(Cow<'s, [u8]>),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn number_array() {
        let events = Parser::new("[1,2,3]")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::BeginNested {
                    name: None,
                    kind: Nested::List
                },
                Event::Primitive(Primitive::Integer(Integer::Usize(1))),
                Event::Primitive(Primitive::Integer(Integer::Usize(2))),
                Event::Primitive(Primitive::Integer(Integer::Usize(3))),
                Event::EndNested,
            ]
        );
        let events = Parser::new("[1,2,3,]")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::BeginNested {
                    name: None,
                    kind: Nested::List
                },
                Event::Primitive(Primitive::Integer(Integer::Usize(1))),
                Event::Primitive(Primitive::Integer(Integer::Usize(2))),
                Event::Primitive(Primitive::Integer(Integer::Usize(3))),
                Event::EndNested,
            ]
        );
    }
    #[test]
    fn number_tuple() {
        let events = Parser::new("(1,2,3)")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::BeginNested {
                    name: None,
                    kind: Nested::Tuple
                },
                Event::Primitive(Primitive::Integer(Integer::Usize(1))),
                Event::Primitive(Primitive::Integer(Integer::Usize(2))),
                Event::Primitive(Primitive::Integer(Integer::Usize(3))),
                Event::EndNested,
            ]
        );
        let events = Parser::new("(1,2,3,)")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::BeginNested {
                    name: None,
                    kind: Nested::Tuple
                },
                Event::Primitive(Primitive::Integer(Integer::Usize(1))),
                Event::Primitive(Primitive::Integer(Integer::Usize(2))),
                Event::Primitive(Primitive::Integer(Integer::Usize(3))),
                Event::EndNested,
            ]
        );
    }
    #[test]
    fn number_map() {
        let events = Parser::new("{a:1,b:2}")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::BeginNested {
                    name: None,
                    kind: Nested::Map
                },
                Event::Primitive(Primitive::Identifier(Cow::Borrowed("a"))),
                Event::Primitive(Primitive::Integer(Integer::Usize(1))),
                Event::Primitive(Primitive::Identifier(Cow::Borrowed("b"))),
                Event::Primitive(Primitive::Integer(Integer::Usize(2))),
                Event::EndNested,
            ]
        );
        let events = Parser::new("{a:1,b:2,}")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::BeginNested {
                    name: None,
                    kind: Nested::Map
                },
                Event::Primitive(Primitive::Identifier(Cow::Borrowed("a"))),
                Event::Primitive(Primitive::Integer(Integer::Usize(1))),
                Event::Primitive(Primitive::Identifier(Cow::Borrowed("b"))),
                Event::Primitive(Primitive::Integer(Integer::Usize(2))),
                Event::EndNested,
            ]
        );
    }
}
