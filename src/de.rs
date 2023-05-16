use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use core::fmt::Display;
use core::ops::Range;

use serde::de::{EnumAccess, MapAccess, SeqAccess, VariantAccess};
use serde::Deserialize;

use crate::parser::{self, Config, Event, EventKind, Name, Nested, Parser, Primitive};
use crate::tokenizer::{self, Integer};

pub struct Deserializer<'de> {
    parser: BetterPeekable<Parser<'de>>,
}

impl<'de> Deserializer<'de> {
    pub fn new(source: &'de str, config: Config) -> Self {
        Self {
            parser: BetterPeekable::new(Parser::new(source, config.include_comments(false))),
        }
    }

    pub fn ensure_eof(mut self) -> Result<(), Error> {
        match self.parser.next() {
            None => Ok(()),
            Some(Ok(event)) => Err(Error::new(event.location, parser::ErrorKind::TrailingData)),
            Some(Err(err)) => Err(err.into()),
        }
    }

    fn handle_unit(&mut self) -> Result<(), DeserializerError> {
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
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedUnit,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedUnit)),
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
                }) => visitor.$visit_name(value.$conv_name().ok_or_else(|| {
                    DeserializerError::new(location, tokenizer::ErrorKind::IntegerTooLarge)
                })?),
                Some(evt) => Err(DeserializerError::new(
                    evt.location,
                    ErrorKind::ExpectedInteger,
                )),
                None => Err(DeserializerError::new(None, ErrorKind::ExpectedInteger)),
            }
        }
    };
}

impl<'de> serde::de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = DeserializerError;

    deserialize_int_impl!(deserialize_i8, visit_i8, as_i8);

    deserialize_int_impl!(deserialize_i16, visit_i16, as_i16);

    deserialize_int_impl!(deserialize_i32, visit_i32, as_i32);

    deserialize_int_impl!(deserialize_i64, visit_i64, as_i64);

    deserialize_int_impl!(deserialize_i128, visit_i128, as_i128);

    deserialize_int_impl!(deserialize_u8, visit_u8, as_u8);

    deserialize_int_impl!(deserialize_u16, visit_u16, as_u16);

    deserialize_int_impl!(deserialize_u32, visit_u32, as_u32);

    deserialize_int_impl!(deserialize_u64, visit_u64, as_u64);

    deserialize_int_impl!(deserialize_u128, visit_u128, as_u128);

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let event = match self.parser.next().transpose()? {
            Some(event) => event,
            None => return visitor.visit_unit(),
        };
        match event.kind {
            EventKind::BeginNested { name, kind } => match kind {
                // Check for Some(), and ensure that this isn't a raw identifier
                // by checking the original token's length.
                Nested::Tuple
                    if name.as_deref() == Some("Some")
                        && event.location.end - event.location.start == 4 =>
                {
                    let value = visitor.visit_some(&mut *self)?;
                    let possible_close = self
                        .parser
                        .next()
                        .transpose()?
                        .expect("parser would error without EndNested");
                    match possible_close.kind {
                        EventKind::EndNested => Ok(value),
                        _ => Err(DeserializerError::new(
                            possible_close.location,
                            ErrorKind::SomeCanOnlyContainOneValue,
                        )),
                    }
                }
                Nested::List | Nested::Tuple => {
                    if matches!(
                        self.parser.peek(),
                        Some(Ok(Event {
                            kind: EventKind::EndNested,
                            ..
                        }))
                    ) {
                        self.parser.next();
                        visitor.visit_unit()
                    } else {
                        visitor.visit_seq(self)
                    }
                }
                Nested::Map => visitor.visit_map(self),
            },
            EventKind::Primitive(primitive) => match primitive {
                Primitive::Bool(v) => visitor.visit_bool(v),
                Primitive::Integer(v) => match v {
                    Integer::Usize(usize) => match usize::BITS {
                        0..=16 => visitor.visit_u16(usize as u16),
                        17..=32 => visitor.visit_u32(usize as u32),
                        33..=64 => visitor.visit_u64(usize as u64),
                        65..=128 => visitor.visit_u128(usize as u128),
                        _ => unreachable!("unsupported pointer width"),
                    },
                    Integer::Isize(isize) => match usize::BITS {
                        0..=16 => visitor.visit_i16(isize as i16),
                        17..=32 => visitor.visit_i32(isize as i32),
                        33..=64 => visitor.visit_i64(isize as i64),
                        65..=128 => visitor.visit_i128(isize as i128),
                        _ => unreachable!("unsupported pointer width"),
                    },
                    #[cfg(feature = "integer128")]
                    Integer::UnsignedLarge(large) => visitor.visit_u128(large),
                    #[cfg(not(feature = "integer128"))]
                    Integer::UnsignedLarge(large) => visitor.visit_u64(large),
                    #[cfg(feature = "integer128")]
                    Integer::SignedLarge(large) => visitor.visit_i128(large),
                    #[cfg(not(feature = "integer128"))]
                    Integer::SignedLarge(large) => visitor.visit_i64(large),
                },
                Primitive::Float(v) => visitor.visit_f64(v),
                Primitive::Char(v) => visitor.visit_char(v),
                Primitive::String(v) => match v {
                    Cow::Borrowed(v) => visitor.visit_borrowed_str(v),
                    Cow::Owned(v) => visitor.visit_string(v),
                },
                Primitive::Identifier(v) => {
                    // The tokenizer will have tokenized `r#None` to `None`, so
                    // we must check the length of the original source to verify
                    // this isn't a raw identifier.
                    if v == "None" && event.location.end - event.location.start == 4 {
                        visitor.visit_none()
                    } else {
                        visitor.visit_borrowed_str(v)
                    }
                }
                Primitive::Bytes(v) => match v {
                    Cow::Borrowed(v) => visitor.visit_borrowed_bytes(v),
                    Cow::Owned(v) => visitor.visit_byte_buf(v),
                },
            },
            EventKind::Comment(_) => unreachable!("comments are disabled"),
            EventKind::EndNested => unreachable!("parser would error"),
        }
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
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedInteger,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedInteger)),
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
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedFloat,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedFloat)),
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
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedChar,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedChar)),
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
                        .map_err(|_| DeserializerError::new(location, ErrorKind::InvalidUtf8))?,
                ),
                Cow::Owned(bytes) => visitor.visit_string(
                    String::from_utf8(bytes)
                        .map_err(|_| DeserializerError::new(location, ErrorKind::InvalidUtf8))?,
                ),
            },
            Some(Event {
                kind:
                    EventKind::BeginNested {
                        name: Some(name), ..
                    },
                ..
            }) => visitor.visit_str(name.name),
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedString,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedString)),
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
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedBytes,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedBytes)),
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
            }) if matches!(name, Some(Name { name: "Some", .. })) => {
                let result = visitor.visit_some(&mut *self)?;
                match self.parser.next().transpose()? {
                    Some(Event {
                        kind: EventKind::EndNested,
                        ..
                    }) => Ok(result),
                    Some(evt) => Err(DeserializerError::new(
                        evt.location,
                        ErrorKind::SomeCanOnlyContainOneValue,
                    )),
                    None => unreachable!("parser errors on early eof"),
                }
            }
            Some(evt) => Err(DeserializerError::new(
                evt.location,
                ErrorKind::ExpectedOption,
            )),
            None => Err(DeserializerError::new(None, ErrorKind::ExpectedOption)),
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
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple_struct(name, 1, visitor)
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
                    return Err(DeserializerError::new(
                        location,
                        ErrorKind::ExpectedSequence,
                    ));
                }

                visitor.visit_seq(self)
            }
            Some(other) => Err(DeserializerError::new(
                other.location,
                ErrorKind::ExpectedSequence,
            )),
            None => Err(DeserializerError::new(
                None,
                parser::ErrorKind::UnexpectedEof,
            )),
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
                if name.map_or(false, |name| name != struct_name) {
                    return Err(DeserializerError::new(
                        location,
                        ErrorKind::NameMismatch(struct_name),
                    ));
                }

                if kind != Nested::Tuple {
                    return Err(DeserializerError::new(
                        location,
                        ErrorKind::ExpectedTupleStruct,
                    ));
                }
            }
            Some(other) => {
                return Err(DeserializerError::new(
                    other.location,
                    ErrorKind::ExpectedTupleStruct,
                ));
            }
            None => {
                return Err(DeserializerError::new(
                    None,
                    parser::ErrorKind::UnexpectedEof,
                ))
            }
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
                    return Err(DeserializerError::new(location, ErrorKind::ExpectedMap));
                }

                visitor.visit_map(self)
            }
            Some(other) => Err(DeserializerError::new(
                other.location,
                ErrorKind::ExpectedMap,
            )),
            None => Err(DeserializerError::new(
                None,
                parser::ErrorKind::UnexpectedEof,
            )),
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
                if name.map_or(false, |name| name != struct_name) {
                    return Err(DeserializerError::new(
                        location,
                        ErrorKind::NameMismatch(struct_name),
                    ));
                }

                if kind != Nested::Map {
                    return Err(DeserializerError::new(
                        location,
                        ErrorKind::ExpectedMapStruct,
                    ));
                }
            }
            Some(other) => {
                return Err(DeserializerError::new(
                    other.location,
                    ErrorKind::ExpectedMapStruct,
                ));
            }
            None => {
                return Err(DeserializerError::new(
                    None,
                    parser::ErrorKind::UnexpectedEof,
                ))
            }
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
                None => {
                    return Err(DeserializerError::new(
                        None,
                        parser::ErrorKind::UnexpectedEof,
                    ))
                }
            }

            if depth == 0 {
                break;
            }
        }

        visitor.visit_unit()
    }
}

impl<'de> MapAccess<'de> for Deserializer<'de> {
    type Error = DeserializerError;

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
            None => Err(DeserializerError::new(
                None,
                parser::ErrorKind::UnexpectedEof,
            )),
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
    type Error = DeserializerError;

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
            None => Err(DeserializerError::new(
                None,
                parser::ErrorKind::UnexpectedEof,
            )),
        }
    }
}

impl<'a, 'de> EnumAccess<'de> for &'a mut Deserializer<'de> {
    type Error = DeserializerError;
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
    type Error = DeserializerError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        let result = seed.deserialize(&mut *self)?;
        loop {
            if let Event {
                kind: EventKind::EndNested,
                ..
            } = self
                .parser
                .next()
                .transpose()?
                .expect("eof handled by parser")
            {
                return Ok(result);
            }
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub location: Range<usize>,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(location: Range<usize>, kind: impl Into<ErrorKind>) -> Self {
        Self {
            location,
            kind: kind.into(),
        }
    }
}

impl From<parser::Error> for Error {
    fn from(err: parser::Error) -> Self {
        Self {
            location: err.location,
            kind: err.kind.into(),
        }
    }
}
impl serde::ser::StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.kind, self.location.start, self.location.end
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeserializerError {
    pub location: Option<Range<usize>>,
    pub kind: ErrorKind,
}

impl DeserializerError {
    pub fn new(location: impl Into<Option<Range<usize>>>, kind: impl Into<ErrorKind>) -> Self {
        Self {
            location: location.into(),
            kind: kind.into(),
        }
    }
}

impl serde::de::Error for DeserializerError {
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

impl serde::ser::StdError for DeserializerError {}

impl Display for DeserializerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(location) = &self.location {
            write!(f, "{} at {}..{}", self.kind, location.start, location.end)
        } else {
            Display::fmt(&self.kind, f)
        }
    }
}

impl From<parser::Error> for DeserializerError {
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

impl Config {
    pub fn deserialize<'de, T: Deserialize<'de>>(self, source: &'de str) -> Result<T, Error> {
        let mut deserializer = Deserializer::new(source, self);
        let result = match T::deserialize(&mut deserializer) {
            Ok(result) => result,
            Err(err) => {
                let location = err
                    .location
                    .unwrap_or_else(|| deserializer.parser.current_range());
                return Err(Error::new(location, err.kind));
            }
        };
        deserializer.ensure_eof()?;
        Ok(result)
    }
}

struct BetterPeekable<T>
where
    T: Iterator,
{
    iter: T,
    peeked: Option<T::Item>,
}

impl<T> BetterPeekable<T>
where
    T: Iterator,
{
    pub fn new(iter: T) -> Self {
        Self { iter, peeked: None }
    }

    pub fn peek(&mut self) -> Option<&T::Item> {
        if self.peeked.is_none() {
            self.peeked = self.next();
        }

        self.peeked.as_ref()
    }
}

impl<T> core::ops::Deref for BetterPeekable<T>
where
    T: Iterator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.iter
    }
}

impl<T> Iterator for BetterPeekable<T>
where
    T: Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.peeked.take().or_else(|| self.iter.next())
    }
}

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
        let config = Config::default().allow_implicit_map(true);
        let parsed = config.deserialize::<BasicNamed>(r#"a: 1 b: -1"#).unwrap();
        assert_eq!(parsed, BasicNamed { a: 1, b: -1 });
        let parsed = config.deserialize::<BasicNamed>(r#"a: 1, b: -1,"#).unwrap();
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
