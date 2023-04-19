use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use core::fmt::Display;
use core::iter::Peekable;
use core::ops::Range;

use serde::de::{EnumAccess, MapAccess, SeqAccess, VariantAccess};
use serde::Deserializer as _;

use crate::parser::{self, Config, Event, EventKind, Nested, Parser, Primitive};
use crate::tokenizer;

pub struct Deserializer<'de> {
    parser: Peekable<Parser<'de>>,
}

impl<'de> Deserializer<'de> {
    pub fn new(source: &'de str, config: Config) -> Self {
        Self {
            parser: Parser::new(source, config.include_comments(false)).peekable(),
        }
    }

    fn handle_unit(&mut self) -> Result<(), Error> {
        match self.parser.next().transpose()? {
            Some(Event {
                kind:
                    EventKind::BeginNested {
                        kind: Nested::Tuple,
                        ..
                    },
                ..
            }) => {
                let mut nests = 1;
                while nests > 0 {
                    match self.parser.next().transpose()? {
                        Some(Event {
                            kind: EventKind::BeginNested { .. },
                            ..
                        }) => nests += 1,
                        Some(Event {
                            kind: EventKind::EndNested,
                            ..
                        }) => nests -= 1,
                        Some(_) => {}
                        None => unreachable!("parser errors on early eof"),
                    }
                }
                Ok(())
            }
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedUnit)),
            None => Err(Error::new(None, ErrorKind::ExpectedUnit)),
        }
    }
}

macro_rules! deserialize_int_impl {
    ($de_name:ident, $visit_name:ident, $conv_name:ident) => {
        fn $de_name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match self.parser.next().transpose()? {
                Some(Event {
                    kind: EventKind::Primitive(Primitive::Integer(value)),
                    location,
                }) => {
                    visitor.$visit_name(value.$conv_name().ok_or_else(|| {
                        Error::new(location, tokenizer::ErrorKind::IntegerTooLarge)
                    })?)
                }
                Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedInteger)),
                None => Err(Error::new(None, ErrorKind::ExpectedInteger)),
            }
        }
    };
}

impl<'de> serde::de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    deserialize_int_impl!(deserialize_i8, visit_i8, into_i8);

    deserialize_int_impl!(deserialize_i16, visit_i16, into_i16);

    deserialize_int_impl!(deserialize_i32, visit_i32, into_i32);

    deserialize_int_impl!(deserialize_i64, visit_i64, into_i64);

    deserialize_int_impl!(deserialize_i128, visit_i128, into_i128);

    deserialize_int_impl!(deserialize_u8, visit_u8, into_u8);

    deserialize_int_impl!(deserialize_u16, visit_u16, into_u16);

    deserialize_int_impl!(deserialize_u32, visit_u32, into_u32);

    deserialize_int_impl!(deserialize_u64, visit_u64, into_u64);

    deserialize_int_impl!(deserialize_u128, visit_u128, into_u128);

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!("implement after serialization is implemented")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::Primitive(Primitive::Bool(value)),
                ..
            }) => visitor.visit_bool(value),
            Some(Event {
                kind: EventKind::Primitive(Primitive::Integer(value)),
                ..
            }) => visitor.visit_bool(!value.is_zero()),
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedInteger)),
            None => Err(Error::new(None, ErrorKind::ExpectedInteger)),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::Primitive(Primitive::Float(value)),
                ..
            }) => visitor.visit_f64(value),
            Some(Event {
                kind: EventKind::Primitive(Primitive::Integer(value)),
                ..
            }) => visitor.visit_f64(value.as_f64()),
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedFloat)),
            None => Err(Error::new(None, ErrorKind::ExpectedFloat)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::Primitive(Primitive::Char(value)),
                ..
            }) => visitor.visit_char(value),
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedChar)),
            None => Err(Error::new(None, ErrorKind::ExpectedChar)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::Primitive(Primitive::Identifier(str)),
                ..
            }) => visitor.visit_borrowed_str(str),
            Some(Event {
                kind: EventKind::Primitive(Primitive::String(str)),
                ..
            }) => match str {
                Cow::Borrowed(str) => visitor.visit_borrowed_str(str),
                Cow::Owned(str) => visitor.visit_string(str),
            },
            Some(Event {
                kind: EventKind::Primitive(Primitive::Bytes(bytes)),
                location,
            }) => match bytes {
                Cow::Borrowed(bytes) => visitor.visit_borrowed_str(
                    core::str::from_utf8(bytes)
                        .map_err(|_| Error::new(location, ErrorKind::InvalidUtf8))?,
                ),
                Cow::Owned(bytes) => visitor.visit_string(
                    String::from_utf8(bytes)
                        .map_err(|_| Error::new(location, ErrorKind::InvalidUtf8))?,
                ),
            },
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedString)),
            None => Err(Error::new(None, ErrorKind::ExpectedString)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::Primitive(Primitive::Identifier(str)),
                ..
            }) => visitor.visit_borrowed_bytes(str.as_bytes()),
            Some(Event {
                kind: EventKind::Primitive(Primitive::String(str)),
                ..
            }) => match str {
                Cow::Borrowed(str) => visitor.visit_borrowed_bytes(str.as_bytes()),
                Cow::Owned(str) => visitor.visit_byte_buf(str.into_bytes()),
            },
            Some(Event {
                kind: EventKind::Primitive(Primitive::Bytes(bytes)),
                ..
            }) => match bytes {
                Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
                Cow::Owned(bytes) => visitor.visit_byte_buf(bytes),
            },
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedBytes)),
            None => Err(Error::new(None, ErrorKind::ExpectedBytes)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::Primitive(Primitive::Identifier(str)),
                ..
            }) if str == "None" => visitor.visit_none(),
            Some(Event {
                kind:
                    EventKind::BeginNested {
                        name,
                        kind: Nested::Tuple,
                    },
                ..
            }) if matches!(name, Some((_, "Some"))) => {
                let result = visitor.visit_some(&mut *self)?;
                match self.parser.next().transpose()? {
                    Some(Event {
                        kind: EventKind::EndNested,
                        ..
                    }) => Ok(result),
                    Some(evt) => Err(Error::new(
                        evt.location,
                        ErrorKind::SomeCanOnlyContainOneValue,
                    )),
                    None => unreachable!("parser errors on early eof"),
                }
            }
            Some(evt) => Err(Error::new(evt.location, ErrorKind::ExpectedOption)),
            None => Err(Error::new(None, ErrorKind::ExpectedOption)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.handle_unit()?;

        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::BeginNested { kind, .. },
                location,
            }) => {
                if !matches!(kind, Nested::Tuple | Nested::List) {
                    return Err(Error::new(location, ErrorKind::ExpectedSequence));
                }

                visitor.visit_seq(self)
            }
            Some(other) => Err(Error::new(other.location, ErrorKind::ExpectedSequence)),
            None => Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        struct_name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::BeginNested { name, kind },
                location,
            }) => {
                if name.map_or(false, |(_, name)| name != struct_name) {
                    return Err(Error::new(location, ErrorKind::NameMismatch(struct_name)));
                }

                if kind != Nested::Tuple {
                    return Err(Error::new(location, ErrorKind::ExpectedTupleStruct));
                }
            }
            Some(other) => {
                return Err(Error::new(other.location, ErrorKind::ExpectedTupleStruct));
            }
            None => return Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
        }

        visitor.visit_seq(self)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::BeginNested { kind, .. },
                location,
            }) => {
                if kind != Nested::Map {
                    return Err(Error::new(location, ErrorKind::ExpectedMap));
                }

                visitor.visit_map(self)
            }
            Some(other) => Err(Error::new(other.location, ErrorKind::ExpectedMap)),
            None => Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
        }
    }

    fn deserialize_struct<V>(
        self,
        struct_name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event {
                kind: EventKind::BeginNested { name, kind },
                location,
            }) => {
                if name.map_or(false, |(_, name)| name != struct_name) {
                    return Err(Error::new(location, ErrorKind::NameMismatch(struct_name)));
                }

                if kind != Nested::Map {
                    return Err(Error::new(location, ErrorKind::ExpectedMapStruct));
                }
            }
            Some(other) => {
                return Err(Error::new(other.location, ErrorKind::ExpectedMapStruct));
            }
            None => return Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
        }

        visitor.visit_map(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let mut depth = 0;
        loop {
            match self.parser.next().transpose()? {
                Some(Event {
                    kind: EventKind::BeginNested { .. },
                    ..
                }) => {
                    depth += 1;
                }
                Some(Event {
                    kind: EventKind::EndNested,
                    ..
                }) => {
                    depth -= 1;
                }
                Some(Event {
                    kind: EventKind::Primitive(_) | EventKind::Comment(_),
                    ..
                }) => {}
                None => return Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
            }

            if depth == 0 {
                break;
            }
        }

        visitor.visit_unit()
    }
}

impl<'de> MapAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.parser.peek() {
            Some(Ok(Event {
                kind: EventKind::EndNested,
                ..
            })) => {
                self.parser.next();
                Ok(None)
            }
            Some(_) => seed.deserialize(self).map(Some),
            None => Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self)
    }
}

impl<'de> SeqAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.parser.peek() {
            Some(Ok(Event {
                kind: EventKind::EndNested,
                ..
            })) => {
                self.parser.next();
                Ok(None)
            }
            Some(_) => seed.deserialize(self).map(Some),
            None => Err(Error::new(None, parser::ErrorKind::UnexpectedEof)),
        }
    }
}

impl<'a, 'de> EnumAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self)?;
        Ok((variant, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.handle_unit()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub location: Option<Range<usize>>,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(location: impl Into<Option<Range<usize>>>, kind: impl Into<ErrorKind>) -> Self {
        Self {
            location: location.into(),
            kind: kind.into(),
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self {
            location: None,
            kind: ErrorKind::Message(msg.to_string()),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(location) = &self.location {
            write!(f, "{} at {}..{}", self.kind, location.start, location.end)
        } else {
            Display::fmt(&self.kind, f)
        }
    }
}

impl From<parser::Error> for Error {
    fn from(err: parser::Error) -> Self {
        Self {
            location: Some(err.location),
            kind: err.kind.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    ExpectedInteger,
    ExpectedFloat,
    ExpectedUnit,
    ExpectedBool,
    ExpectedOption,
    ExpectedChar,
    ExpectedString,
    ExpectedBytes,
    ExpectedSequence,
    ExpectedMap,
    ExpectedTupleStruct,
    ExpectedMapStruct,
    InvalidUtf8,
    NameMismatch(&'static str),
    SomeCanOnlyContainOneValue,
    Parser(parser::ErrorKind),
    Message(String),
}

impl From<parser::ErrorKind> for ErrorKind {
    fn from(kind: parser::ErrorKind) -> Self {
        Self::Parser(kind)
    }
}

impl From<tokenizer::ErrorKind> for ErrorKind {
    fn from(kind: tokenizer::ErrorKind) -> Self {
        Self::Parser(kind.into())
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorKind::Parser(parser) => Display::fmt(parser, f),
            ErrorKind::Message(message) => f.write_str(message),
            ErrorKind::ExpectedInteger => f.write_str("expected integer"),
            ErrorKind::ExpectedFloat => f.write_str("expected float"),
            ErrorKind::ExpectedBool => f.write_str("expected bool"),
            ErrorKind::ExpectedUnit => f.write_str("expected unit"),
            ErrorKind::ExpectedOption => f.write_str("expected option"),
            ErrorKind::ExpectedChar => f.write_str("expected char"),
            ErrorKind::ExpectedString => f.write_str("expected string"),
            ErrorKind::ExpectedBytes => f.write_str("expected bytes"),
            ErrorKind::SomeCanOnlyContainOneValue => {
                f.write_str("Some(_) can only contain one value")
            }
            ErrorKind::ExpectedSequence => f.write_str("expected sequence"),
            ErrorKind::ExpectedMap => f.write_str("expected map"),
            ErrorKind::ExpectedTupleStruct => f.write_str("expected tuple struct"),
            ErrorKind::ExpectedMapStruct => f.write_str("expected map struct"),
            ErrorKind::NameMismatch(name) => write!(f, "name mismatch, expected {name}"),
            ErrorKind::InvalidUtf8 => f.write_str("invalid utf-8"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::parser::Config;

    #[test]
    fn basic_named() {
        #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
        struct BasicNamed {
            a: u32,
            b: i32,
        }

        let parsed = crate::from_str::<BasicNamed>(r#"BasicNamed{ a: 1, b: -1 }"#).unwrap();
        assert_eq!(parsed, BasicNamed { a: 1, b: -1 });
    }

    #[test]
    fn implicit_map() {
        #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
        struct BasicNamed {
            a: u32,
            b: i32,
        }
        let parsed = BasicNamed::deserialize(&mut crate::de::Deserializer::new(
            r#"a: 1 b: -1"#,
            Config::default().allow_implicit_map(true),
        ))
        .unwrap();
        assert_eq!(parsed, BasicNamed { a: 1, b: -1 });
        let parsed = BasicNamed::deserialize(&mut crate::de::Deserializer::new(
            r#"a: 1, b: -1,"#,
            Config::default().allow_implicit_map(true),
        ))
        .unwrap();
        assert_eq!(parsed, BasicNamed { a: 1, b: -1 });
    }

    #[test]
    fn optional() {
        #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
        struct BasicNamed {
            a: u32,
            b: i32,
        }

        assert_eq!(
            crate::from_str::<Option<BasicNamed>>(r#"None"#).unwrap(),
            None
        );

        let parsed =
            crate::from_str::<Option<BasicNamed>>(r#"Some(BasicNamed{ a: 1, b: -1 })"#).unwrap();
        assert_eq!(parsed, Some(BasicNamed { a: 1, b: -1 }));
    }
}
