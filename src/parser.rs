use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use core::mem;
use core::ops::{Deref, Range};

use crate::tokenizer::{self, Balanced, Integer, Token, TokenKind, Tokenizer};

#[derive(Debug)]
pub struct Parser<'s> {
    tokens: Tokenizer<'s, false>,
    peeked: Option<Result<Token<'s>, tokenizer::Error>>,
    nested: Vec<(usize, NestedState)>,
    root_state: State<'s>,
    config: Config,
}

impl<'s> Parser<'s> {
    #[must_use]
    pub fn new(source: &'s str, config: Config) -> Self {
        Self {
            tokens: Tokenizer::minified(source),
            peeked: None,
            nested: Vec::new(),
            root_state: State::AtStart,
            config,
        }
    }

    #[must_use]
    pub fn validate(source: &'s str, config: Config) -> bool {
        Self::new(source, config).all(|result| result.is_ok())
    }

    #[must_use]
    pub const fn current_offset(&self) -> usize {
        self.tokens.current_offset()
    }

    #[must_use]
    pub fn current_range(&self) -> Range<usize> {
        let start = self.nested.last().map_or(0, |(offset, _)| *offset);
        start..self.tokens.current_offset()
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

    fn next_token_parts(
        &mut self,
    ) -> Result<(Range<usize>, Option<TokenKind<'s>>), tokenizer::Error> {
        Ok(match self.next_token().transpose()? {
            Some(token) => (token.location, Some(token.kind)),
            None => (
                self.tokens.current_offset()..self.tokens.current_offset(),
                None,
            ),
        })
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
            TokenKind::Integer(integer) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Integer(integer)),
            )),
            TokenKind::Float(float) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Float(float)),
            )),
            TokenKind::Bool(value) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Bool(value)),
            )),
            TokenKind::Character(value) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Char(value)),
            )),
            TokenKind::Byte(value) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Integer(Integer::Usize(value as usize))),
            )),
            TokenKind::String(value) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::String(value)),
            )),
            TokenKind::Bytes(value) => Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Bytes(value)),
            )),
            TokenKind::Identifier(value) => self.parse_identifier(token, value),
            TokenKind::Open(Balanced::Paren) => {
                self.nested.push((
                    token.location.start,
                    NestedState::Tuple(ListState::ExpectingValue),
                ));
                Ok(Event::new(
                    token.location,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Tuple,
                    },
                ))
            }
            TokenKind::Open(Balanced::Bracket) => {
                self.nested.push((
                    token.location.start,
                    NestedState::List(ListState::ExpectingValue),
                ));
                Ok(Event::new(
                    token.location,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::List,
                    },
                ))
            }
            TokenKind::Open(Balanced::Brace) => {
                self.nested.push((
                    token.location.start,
                    NestedState::Map(MapState::ExpectingKey),
                ));
                Ok(Event::new(
                    token.location,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Map,
                    },
                ))
            }
            TokenKind::Close(closed) if Some(closed) == allowed_close => {
                self.nested.pop();
                Ok(Event::new(token.location, EventKind::EndNested))
            }
            TokenKind::Colon | TokenKind::Comma | TokenKind::Close(_) => {
                Err(Error::new(token.location, ErrorKind::ExpectedValue))
            }
            TokenKind::Comment(comment) => {
                Ok(Event::new(token.location, EventKind::Comment(comment)))
            }
            TokenKind::Whitespace(_) => unreachable!("disabled"),
        }
    }

    fn parse_identifier(&mut self, token: Token<'s>, value: &'s str) -> Result<Event<'s>, Error> {
        if matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Open(Balanced::Brace | Balanced::Paren),
                ..
            })
        ) {
            let Some(Ok(Token {
                kind: TokenKind::Open(balanced),
                location: open_location,
            })) = self.next_token()
            else {
                unreachable!("matched above")
            };

            let kind = match balanced {
                Balanced::Paren => {
                    self.nested.push((
                        open_location.start,
                        NestedState::Tuple(ListState::ExpectingValue),
                    ));
                    Nested::Tuple
                }
                Balanced::Brace => {
                    self.nested.push((
                        open_location.start,
                        NestedState::Map(MapState::ExpectingKey),
                    ));
                    Nested::Map
                }
                Balanced::Bracket => {
                    unreachable!("specifically excluded above")
                }
            };

            Ok(Event::new(
                open_location,
                EventKind::BeginNested {
                    name: Some(Name {
                        location: token.location,
                        name: value,
                    }),
                    kind,
                },
            ))
        } else if matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Open(Balanced::Bracket),
                ..
            })
        ) {
            let location = self.peek().expect("just matched").location.clone();
            return Err(Error::new(location, ErrorKind::ExpectedMapOrTuple));
        } else {
            Ok(Event::new(
                token.location,
                EventKind::Primitive(Primitive::Identifier(value)),
            ))
        }
    }

    fn parse_sequence(&mut self, state: ListState, end: Balanced) -> Result<Event<'s>, Error> {
        match state {
            ListState::ExpectingValue => {
                let token = self.next_or_eof()?;
                if let TokenKind::Comment(comment) = &token.kind {
                    Ok(Event::new(token.location, EventKind::Comment(comment)))
                } else {
                    self.nested.last_mut().expect("required for this fn").1 =
                        NestedState::list(end, ListState::ExpectingComma);
                    self.parse_token(token, Some(end))
                }
            }
            ListState::ExpectingComma => match self.next_token_parts()? {
                (location, Some(TokenKind::Close(closed))) if closed == end => {
                    self.nested.pop();
                    Ok(Event::new(location, EventKind::EndNested))
                }
                (_, Some(TokenKind::Comma)) => {
                    self.nested.last_mut().expect("required for this fn").1 =
                        NestedState::list(end, ListState::ExpectingValue);
                    self.parse_sequence(ListState::ExpectingValue, end)
                }
                (location, Some(TokenKind::Comment(comment))) => {
                    Ok(Event::new(location, EventKind::Comment(comment)))
                }
                (location, _) => Err(Error::new(
                    location,
                    ErrorKind::ExpectedCommaOrEnd(end.into()),
                )),
            },
        }
    }

    fn map_state_mut(&mut self) -> &mut MapState {
        let Some((_, NestedState::Map(map_state))) = self.nested.last_mut() else {
            unreachable!("not a map state")
        };
        map_state
    }

    fn parse_map(&mut self, state: MapState) -> Result<Event<'s>, Error> {
        match state {
            MapState::ExpectingKey => match self.next_token().transpose()? {
                Some(Token {
                    kind: TokenKind::Comment(comment),
                    location,
                }) => Ok(Event::new(location, EventKind::Comment(comment))),
                Some(token) => {
                    *self.map_state_mut() = MapState::ExpectingColon;
                    self.parse_token(token, Some(Balanced::Brace))
                }
                None => Err(Error::new(
                    self.tokens.current_offset()..self.tokens.current_offset(),
                    ErrorKind::ExpectedKey,
                )),
            },
            MapState::ExpectingColon => match self.next_token_parts()? {
                (_, Some(TokenKind::Colon)) => {
                    *self.map_state_mut() = MapState::ExpectingValue;
                    self.parse_map(MapState::ExpectingValue)
                }
                (location, Some(TokenKind::Comment(comment))) => {
                    Ok(Event::new(location, EventKind::Comment(comment)))
                }
                (location, _) => Err(Error::new(location, ErrorKind::ExpectedColon)),
            },
            MapState::ExpectingValue => match self.next_token().transpose()? {
                Some(Token {
                    kind: TokenKind::Comment(comment),
                    location,
                }) => Ok(Event::new(location, EventKind::Comment(comment))),
                Some(token) => {
                    *self.map_state_mut() = MapState::ExpectingComma;
                    self.parse_token(token, None)
                }
                None => Err(Error::new(
                    self.tokens.current_offset()..self.tokens.current_offset(),
                    ErrorKind::ExpectedValue,
                )),
            },
            MapState::ExpectingComma => match self.next_token_parts()? {
                (location, Some(TokenKind::Close(Balanced::Brace))) => {
                    self.nested.pop();
                    Ok(Event::new(location, EventKind::EndNested))
                }
                (_, Some(TokenKind::Comma)) => {
                    *self.map_state_mut() = MapState::ExpectingKey;
                    self.parse_map(MapState::ExpectingKey)
                }
                (location, Some(TokenKind::Comment(comment))) => {
                    Ok(Event::new(location, EventKind::Comment(comment)))
                }
                (location, _) => Err(Error::new(
                    location,
                    ErrorKind::ExpectedCommaOrEnd(Nested::Map),
                )),
            },
        }
    }

    fn parse_implicit_map(&mut self, state: MapState) -> Result<Event<'s>, Error> {
        match state {
            MapState::ExpectingKey => match self.next_token().transpose()? {
                Some(Token {
                    location,
                    kind: TokenKind::Comment(comment),
                }) => Ok(Event::new(location, EventKind::Comment(comment))),
                Some(token) => match self.parse_token(token, None)? {
                    Event {
                        kind: EventKind::Primitive(primitive),
                        location,
                    } => {
                        self.root_state = State::ImplicitMap(MapState::ExpectingColon);
                        Ok(Event::new(location, EventKind::Primitive(primitive)))
                    }
                    Event { location, .. } => Err(Error::new(location, ErrorKind::ExpectedKey)),
                },
                None => {
                    self.root_state = State::Finished;
                    Ok(Event::new(self.current_range(), EventKind::EndNested))
                }
            },
            MapState::ExpectingColon => match self.next_token_parts()? {
                (_, Some(TokenKind::Colon)) => {
                    self.root_state = State::ImplicitMap(MapState::ExpectingValue);
                    self.parse_implicit_map(MapState::ExpectingValue)
                }
                (location, Some(TokenKind::Comment(comment))) => {
                    Ok(Event::new(location, EventKind::Comment(comment)))
                }
                (location, _) => Err(Error::new(location, ErrorKind::ExpectedColon)),
            },
            MapState::ExpectingValue => match self.next_token().transpose()? {
                Some(Token {
                    kind: TokenKind::Comment(comment),
                    location,
                }) => Ok(Event::new(location, EventKind::Comment(comment))),
                Some(token) => {
                    self.root_state = State::ImplicitMap(MapState::ExpectingComma);
                    self.parse_token(token, None)
                }
                None => Err(Error::new(
                    self.tokens.current_offset()..self.tokens.current_offset(),
                    ErrorKind::ExpectedValue,
                )),
            },
            MapState::ExpectingComma => match self.next_token().transpose()? {
                Some(Token {
                    location,
                    kind: TokenKind::Comment(comment),
                }) => Ok(Event::new(location, EventKind::Comment(comment))),
                Some(Token {
                    location,
                    kind: TokenKind::Close(Balanced::Brace),
                }) => {
                    self.root_state = State::Finished;
                    Ok(Event::new(location, EventKind::EndNested))
                }
                Some(Token {
                    kind: TokenKind::Comma,
                    ..
                }) => {
                    self.root_state = State::ImplicitMap(MapState::ExpectingKey);
                    self.parse_implicit_map(MapState::ExpectingKey)
                }
                Some(token) => {
                    self.root_state = State::ImplicitMap(MapState::ExpectingColon);
                    match self.parse_token(token, None)? {
                        Event {
                            location,
                            kind: EventKind::Primitive(primitive),
                        } => Ok(Event::new(location, EventKind::Primitive(primitive))),
                        Event { location, .. } => Err(Error::new(location, ErrorKind::ExpectedKey)),
                    }
                }
                None => {
                    self.root_state = State::Finished;
                    Ok(Event::new(self.current_range(), EventKind::EndNested))
                }
            },
        }
    }

    fn next_event(&mut self) -> Option<Result<Event<'s>, Error>> {
        Some(match self.nested.last() {
            None => match &self.root_state {
                State::AtStart => {
                    let token = match self.next_token()? {
                        Ok(token) => token,
                        Err(err) => return Some(Err(err.into())),
                    };
                    match &token.kind {
                        TokenKind::Comment(comment) => {
                            Ok(Event::new(token.location, EventKind::Comment(comment)))
                        }
                        _ if self.config.allow_implicit_map_at_root
                            && matches!(
                                self.peek(),
                                Some(Token {
                                    kind: TokenKind::Colon,
                                    ..
                                })
                            ) =>
                        {
                            match self.parse_token(token, None) {
                                Ok(event) => {
                                    self.root_state = State::StartingImplicitMap(event);
                                    Ok(Event::new(
                                        0..0,
                                        EventKind::BeginNested {
                                            name: None,
                                            kind: Nested::Map,
                                        },
                                    ))
                                }
                                Err(err) => Err(err),
                            }
                        }
                        _ => {
                            self.root_state = State::Finished;
                            self.parse_token(token, None)
                        }
                    }
                }
                State::StartingImplicitMap(_) => {
                    let State::StartingImplicitMap(event) = mem::replace(
                        &mut self.root_state,
                        State::ImplicitMap(MapState::ExpectingColon),
                    ) else {
                        unreachable!("just matched")
                    };
                    Ok(event)
                }
                State::ImplicitMap(state) => self.parse_implicit_map(*state),
                State::Finished => match self.next_token()? {
                    Ok(token) => match token.kind {
                        TokenKind::Comment(comment) => {
                            Ok(Event::new(token.location, EventKind::Comment(comment)))
                        }
                        TokenKind::Whitespace(_) => unreachable!("disabled"),
                        _ => Err(Error::new(token.location, ErrorKind::TrailingData)),
                    },
                    Err(err) => Err(err.into()),
                },
            },

            Some((_, NestedState::Tuple(list))) => self.parse_sequence(*list, Balanced::Paren),
            Some((_, NestedState::List(list))) => self.parse_sequence(*list, Balanced::Bracket),
            Some((_, NestedState::Map(map))) => self.parse_map(*map),
        })
    }
}

impl<'s> Iterator for Parser<'s> {
    type Item = Result<Event<'s>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let event = self.next_event()?;
            if self.config.include_comments
                || !matches!(
                    event,
                    Ok(Event {
                        kind: EventKind::Comment(_),
                        ..
                    })
                )
            {
                break Some(event);
            }

            // Eat the comment
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Config {
    pub allow_implicit_map_at_root: bool,
    pub include_comments: bool,
}

impl Config {
    #[must_use]
    pub const fn allow_implicit_map_at_root(mut self, allow: bool) -> Self {
        self.allow_implicit_map_at_root = allow;
        self
    }

    #[must_use]
    pub const fn include_comments(mut self, include: bool) -> Self {
        self.include_comments = include;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
enum State<'s> {
    AtStart,
    StartingImplicitMap(Event<'s>),
    ImplicitMap(MapState),
    Finished,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    pub location: Range<usize>,
    pub kind: ErrorKind,
}

impl Error {
    #[must_use]
    pub fn new(location: Range<usize>, kind: ErrorKind) -> Self {
        Self { location, kind }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.kind, f)
    }
}

impl From<tokenizer::Error> for Error {
    fn from(err: tokenizer::Error) -> Self {
        Self {
            location: err.location,
            kind: err.kind.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    Tokenizer(tokenizer::ErrorKind),
    UnexpectedEof,
    ExpectedKey,
    ExpectedColon,
    ExpectedValue,
    ExpectedCommaOrEnd(Nested),
    ExpectedMapOrTuple,
    TrailingData,
}

impl From<tokenizer::ErrorKind> for ErrorKind {
    fn from(kind: tokenizer::ErrorKind) -> Self {
        Self::Tokenizer(kind)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorKind::Tokenizer(err) => Display::fmt(err, f),
            ErrorKind::UnexpectedEof => f.write_str("unexpected end of file"),
            ErrorKind::ExpectedValue => f.write_str("a value was expected"),
            ErrorKind::ExpectedCommaOrEnd(nested) => {
                write!(f, "expected `,` or {}", nested.err_display())
            }
            ErrorKind::ExpectedColon => f.write_str("expected `:`"),
            ErrorKind::ExpectedKey => f.write_str("expected map key"),
            ErrorKind::TrailingData => f.write_str(
                "source contained extra trailing data after a value was completely read",
            ),
            ErrorKind::ExpectedMapOrTuple => {
                f.write_str("[ is not valid for a named value, expected { or (")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event<'s> {
    pub location: Range<usize>,
    pub kind: EventKind<'s>,
}

impl<'s> Event<'s> {
    #[must_use]
    pub fn new(location: Range<usize>, kind: EventKind<'s>) -> Self {
        Self { location, kind }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum EventKind<'s> {
    BeginNested {
        name: Option<Name<'s>>,
        kind: Nested,
    },
    EndNested,
    Primitive(Primitive<'s>),
    Comment(&'s str),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Name<'s> {
    pub location: Range<usize>,
    pub name: &'s str,
}

impl<'s> Deref for Name<'s> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.name
    }
}

impl<'s> PartialEq<str> for Name<'s> {
    fn eq(&self, other: &str) -> bool {
        self.name == other
    }
}

impl<'a, 's> PartialEq<&'a str> for Name<'s> {
    fn eq(&self, other: &&'a str) -> bool {
        self.name == *other
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Nested {
    Tuple,
    Map,
    List,
}

impl Nested {
    fn err_display(self) -> &'static str {
        match self {
            Nested::Tuple => "`)`",
            Nested::Map => "`}`",
            Nested::List => "`]`",
        }
    }
}

impl From<Balanced> for Nested {
    fn from(kind: Balanced) -> Self {
        match kind {
            Balanced::Paren => Self::Tuple,
            Balanced::Bracket => Self::List,
            Balanced::Brace => Self::Map,
        }
    }
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
    Identifier(&'s str),
    Bytes(Cow<'s, [u8]>),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn number_array() {
        let events = Parser::new("[1,2,3]", Config::default())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(
                    0..1,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::List
                    }
                ),
                Event::new(
                    1..2,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(
                    3..4,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(
                    5..6,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(3)))
                ),
                Event::new(6..7, EventKind::EndNested),
            ]
        );
        let events = Parser::new("[1,2,3,]", Config::default())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(
                    0..1,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::List
                    }
                ),
                Event::new(
                    1..2,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(
                    3..4,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(
                    5..6,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(3)))
                ),
                Event::new(7..8, EventKind::EndNested),
            ]
        );
    }

    #[test]
    fn number_tuple() {
        let events = Parser::new("(1,2,3)", Config::default())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(
                    0..1,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Tuple
                    }
                ),
                Event::new(
                    1..2,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(
                    3..4,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(
                    5..6,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(3)))
                ),
                Event::new(6..7, EventKind::EndNested),
            ]
        );
        let events = Parser::new("(1,2,3,)", Config::default())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(
                    0..1,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Tuple
                    }
                ),
                Event::new(
                    1..2,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(
                    3..4,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(
                    5..6,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(3)))
                ),
                Event::new(7..8, EventKind::EndNested),
            ]
        );
    }

    #[test]
    fn number_map() {
        let events = Parser::new("{a:1,b:2}", Config::default())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(
                    0..1,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Map
                    }
                ),
                Event::new(1..2, EventKind::Primitive(Primitive::Identifier("a"))),
                Event::new(
                    3..4,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(5..6, EventKind::Primitive(Primitive::Identifier("b"))),
                Event::new(
                    7..8,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(8..9, EventKind::EndNested),
            ]
        );
        let events = Parser::new("{a:1,b:2,}", Config::default())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(
                    0..1,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Map
                    }
                ),
                Event::new(1..2, EventKind::Primitive(Primitive::Identifier("a"))),
                Event::new(
                    3..4,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(5..6, EventKind::Primitive(Primitive::Identifier("b"))),
                Event::new(
                    7..8,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(9..10, EventKind::EndNested),
            ]
        );
    }

    #[test]
    fn commented() {
        let events = Parser::new(
            "/**/{/**/a/**/:/**/1/**/,/**/b/**/:/**/[/**/2/**/,/**/3/**/]/**/}/**/",
            Config::default().include_comments(true),
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(
            &events,
            &[
                Event::new(0..4, EventKind::Comment("/**/")),
                Event::new(
                    4..5,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::Map
                    }
                ),
                Event::new(5..9, EventKind::Comment("/**/")),
                Event::new(9..10, EventKind::Primitive(Primitive::Identifier("a"))),
                Event::new(10..14, EventKind::Comment("/**/")),
                Event::new(15..19, EventKind::Comment("/**/")),
                Event::new(
                    19..20,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(1)))
                ),
                Event::new(20..24, EventKind::Comment("/**/")),
                Event::new(25..29, EventKind::Comment("/**/")),
                Event::new(29..30, EventKind::Primitive(Primitive::Identifier("b"))),
                Event::new(30..34, EventKind::Comment("/**/")),
                Event::new(35..39, EventKind::Comment("/**/")),
                Event::new(
                    39..40,
                    EventKind::BeginNested {
                        name: None,
                        kind: Nested::List
                    }
                ),
                Event::new(40..44, EventKind::Comment("/**/")),
                Event::new(
                    44..45,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(2)))
                ),
                Event::new(45..49, EventKind::Comment("/**/")),
                Event::new(50..54, EventKind::Comment("/**/")),
                Event::new(
                    54..55,
                    EventKind::Primitive(Primitive::Integer(Integer::Usize(3)))
                ),
                Event::new(55..59, EventKind::Comment("/**/")),
                Event::new(59..60, EventKind::EndNested),
                Event::new(60..64, EventKind::Comment("/**/")),
                Event::new(64..65, EventKind::EndNested),
                Event::new(65..69, EventKind::Comment("/**/")),
            ]
        );
    }

    #[test]
    fn array_named_error() {
        let err = Parser::new("Foo[]", Config::default())
            .next()
            .unwrap()
            .unwrap_err();
        assert_eq!(err, Error::new(3..4, ErrorKind::ExpectedMapOrTuple));
    }
}
