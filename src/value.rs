use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt::Display;
use core::str::{self, FromStr};

use crate::parser::{Config, Error, ErrorKind, Event, EventKind, Name, Nested, Parser, Primitive};
use crate::tokenizer::Integer;
use crate::writer::{self, Writer};

/// A value with a static lifetime.
pub type OwnedValue = Value<'static>;

/// A value representable by Rsn.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// An integer.
    Integer(Integer),
    /// A floating point number.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// A byte.
    Byte(u8),
    /// An identifier (name).
    Identifier(Cow<'a, str>),
    /// A string.
    String(Cow<'a, str>),
    /// A byte string.
    Bytes(Cow<'a, [u8]>),
    /// A named structure.
    Named(Named<'a>),
    /// A tuple of values.
    Tuple(List<'a>),
    /// An array of values.
    Array(List<'a>),
    /// A collection of key-value pairs.
    Map(Map<'a>),
}

macro_rules! as_integer {
    ($name:ident, $ty:ty) => {
        /// Returns this value as a
        #[doc = stringify!($type)]
        /// if the value is an integer that can fit in a
        #[doc = stringify!($type)]
        #[must_use]
        pub fn $name(&self) -> Option<$ty> {
            let Self::Integer(value) = self else {
                return None;
            };

            value.$name()
        }
    };
}

impl<'a> Value<'a> {
    as_integer!(as_u8, u8);

    as_integer!(as_u16, u16);

    as_integer!(as_u32, u32);

    as_integer!(as_u64, u64);

    as_integer!(as_u128, u128);

    as_integer!(as_usize, usize);

    as_integer!(as_i8, i8);

    as_integer!(as_i16, i16);

    as_integer!(as_i32, i32);

    as_integer!(as_i64, i64);

    as_integer!(as_i128, i128);

    as_integer!(as_isize, isize);

    /// Parses `source` as a [`Value`].
    ///
    /// # Errors
    ///
    /// Returns any error encountered while parsing `source`.
    pub fn from_str(source: &'a str, config: Config) -> Result<Self, Error> {
        let mut parser = Parser::new(source, config.include_comments(false));
        Self::parse(&mut parser)
    }

    /// Returns a value representing the unit type.
    #[must_use]
    pub const fn unit() -> Self {
        Self::Tuple(List::new())
    }

    fn parse(parser: &mut Parser<'a>) -> Result<Self, Error> {
        let event = parser.next().transpose()?.ok_or_else(|| {
            Error::new(
                parser.current_offset()..parser.current_offset(),
                ErrorKind::UnexpectedEof,
            )
        })?;
        Self::from_parser_event(event, parser)
    }

    fn from_parser_event(event: Event<'a>, parser: &mut Parser<'a>) -> Result<Self, Error> {
        match event.kind {
            EventKind::BeginNested {
                name,
                kind: kind @ (Nested::Tuple | Nested::List),
            } => Self::parse_sequence(name, parser, kind),
            EventKind::BeginNested {
                name,
                kind: Nested::Map,
            } => Self::parse_map(name, parser),
            EventKind::Primitive(primitive) => match primitive {
                Primitive::Bool(value) => Ok(Value::Bool(value)),
                Primitive::Integer(value) => Ok(Value::Integer(value)),
                Primitive::Float(value) => Ok(Value::Float(value)),
                Primitive::Char(value) => Ok(Value::Char(value)),
                Primitive::String(value) => Ok(Value::String(value)),
                Primitive::Identifier(value) => Ok(Value::Identifier(Cow::Borrowed(value))),
                Primitive::Bytes(value) => Ok(Value::Bytes(value)),
            },
            EventKind::Comment(_) => unreachable!("disabled in parser"),
            EventKind::EndNested => unreachable!("Parser would error"),
        }
    }

    fn parse_sequence(
        name: Option<Name<'a>>,
        parser: &mut Parser<'a>,
        kind: Nested,
    ) -> Result<Self, Error> {
        let mut list = List::default();
        loop {
            let event = parser.next().expect("will error or have another event")?;
            if matches!(event.kind, EventKind::EndNested) {
                if let Some(name) = name {
                    return Ok(Self::Named(Named {
                        name: Cow::Borrowed(name.name),
                        contents: StructContents::Tuple(list),
                    }));
                }

                match kind {
                    Nested::List => return Ok(Self::Array(list)),
                    Nested::Tuple => return Ok(Self::Tuple(list)),
                    Nested::Map => unreachable!("parse_sequence isn't called on maps"),
                }
            } else {
                list.0.push(Self::from_parser_event(event, parser)?);
            }
        }
    }

    fn parse_map(name: Option<Name<'a>>, parser: &mut Parser<'a>) -> Result<Self, Error> {
        let mut map = Map::default();
        loop {
            let event = parser.next().expect("will error or have another event")?;
            if matches!(event.kind, EventKind::EndNested) {
                if let Some(name) = name {
                    return Ok(Self::Named(Named {
                        name: Cow::Borrowed(name.name),
                        contents: StructContents::Map(map),
                    }));
                }

                return Ok(Self::Map(map));
            }

            let key = Self::from_parser_event(event, parser)?;
            let value = Self::from_parser_event(
                parser.next().expect("will error or have another event")?,
                parser,
            )?;

            map.0.push((key, value));
        }
    }

    /// Creates a value by serializing `value` using Serde.
    ///
    /// # Errors
    ///
    /// Returns an error if the `value` cannot be represented losslessly or if
    /// any errors occur from `Serializer` implementations.
    #[cfg(feature = "serde")]
    pub fn from_serialize<S: ::serde::Serialize>(value: &S) -> Result<Self, ToValueError> {
        value.serialize(serde::ValueSerializer)
    }

    /// Deserializes `self` as `D` using Serde.
    ///
    /// # Errors
    ///
    /// Returns an error if `self` cannot be deserialized as `D`.
    #[cfg(feature = "serde")]
    pub fn to_deserialize<D: ::serde::Deserialize<'a>>(&self) -> Result<D, serde::FromValueError> {
        D::deserialize(serde::ValueDeserializer(self))
    }

    /// Returns the owned version of `self`, copying any borrowed data to the
    /// heap.
    #[must_use]
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Integer(value) => Value::Integer(value),
            Value::Float(value) => Value::Float(value),
            Value::Bool(value) => Value::Bool(value),
            Value::Char(value) => Value::Char(value),
            Value::Byte(value) => Value::Byte(value),
            Value::Identifier(value) => Value::Identifier(Cow::Owned(value.into_owned())),
            Value::String(value) => Value::String(Cow::Owned(value.into_owned())),
            Value::Bytes(value) => Value::Bytes(Cow::Owned(value.into_owned())),
            Value::Named(value) => Value::Named(value.into_owned()),
            Value::Tuple(value) => Value::Tuple(value.into_owned()),
            Value::Array(value) => Value::Array(value.into_owned()),
            Value::Map(value) => Value::Map(value.into_owned()),
        }
    }

    /// Returns this value as a floating point number.
    ///
    /// If this is an integer, this will cast the integer to an f64.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Integer(integer) => Some(integer.as_f64()),
            Value::Float(float) => Some(*float),
            _ => None,
        }
    }

    /// Returns this value as a str, if possible.
    ///
    /// `Identifier`, `String`, and `Bytes` bytes all can be returned from this
    /// function.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Identifier(str) | Value::String(str) => Some(str),
            Value::Bytes(bytes) => str::from_utf8(bytes).ok(),
            _ => None,
        }
    }

    /// Returns the underlying bytes for this value, if it can be represented as
    /// a byte slice.
    ///
    /// `Identifier`, `String`, and `Bytes` bytes all can be returned from this
    /// function.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Identifier(str) | Value::String(str) => Some(str.as_bytes()),
            Value::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }
}

impl FromStr for Value<'static> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Value::from_str(s, Config::default()).map(Value::into_owned)
    }
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut writer = if f.alternate() {
            Writer::new(
                f,
                &writer::Config::Pretty {
                    indentation: Cow::Borrowed("  "),
                    newline: Cow::Borrowed("\n"),
                },
            )
        } else {
            Writer::new(f, &writer::Config::Compact)
        };
        writer.write_value(self)?;
        writer.finish();
        Ok(())
    }
}

/// A named structure.
#[derive(Debug, Clone, PartialEq)]
pub struct Named<'a> {
    /// The name of the structure.
    pub name: Cow<'a, str>,
    /// The contents of the structure.
    pub contents: StructContents<'a>,
}

impl<'a> Named<'a> {
    /// Returns an owned representation of this name, copying to the stack if
    /// needed.
    #[must_use]
    pub fn into_owned(self) -> Named<'static> {
        Named {
            name: Cow::Owned(self.name.into_owned()),
            contents: self.contents.into_owned(),
        }
    }
}

/// The contents of a structure.
#[derive(Debug, Clone, PartialEq)]
pub enum StructContents<'a> {
    /// Named fields, represented as a map.
    Map(Map<'a>),
    /// A tuple of valuees.
    Tuple(List<'a>),
}

impl<'a> StructContents<'a> {
    /// Returns an owned representation, copying to the heap if needed.
    #[must_use]
    pub fn into_owned(self) -> StructContents<'static> {
        match self {
            StructContents::Map(contents) => StructContents::Map(contents.into_owned()),
            StructContents::Tuple(contents) => StructContents::Tuple(contents.into_owned()),
        }
    }
}

/// A list of key-value pairs.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Map<'a>(pub Vec<(Value<'a>, Value<'a>)>);

impl<'a> Map<'a> {
    /// Returns an owned representation, copying to the heap if needed.
    #[must_use]
    pub fn into_owned(self) -> Map<'static> {
        Map(self
            .0
            .into_iter()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect())
    }
}

/// A list of values.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct List<'a>(pub Vec<Value<'a>>);

impl<'a> List<'a> {
    /// Returns an empty list.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns an owned representation, copying to the heap if needed.
    #[must_use]
    pub fn into_owned(self) -> List<'static> {
        List(self.0.into_iter().map(Value::into_owned).collect())
    }
}

#[cfg(feature = "serde")]
mod serde {
    use alloc::borrow::Cow;
    use alloc::string::{String, ToString};
    use alloc::{slice, vec};
    use core::fmt::Display;
    use core::num::TryFromIntError;
    use core::str::{self, Utf8Error};

    use serde::de::{EnumAccess, MapAccess, SeqAccess, VariantAccess};
    use serde::ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    };
    use serde::{Deserializer, Serializer};

    use super::{List, StructContents};
    use crate::parser::Nested;
    use crate::tokenizer::Integer;
    use crate::value::{Map, Named, OwnedValue, Value};

    pub struct ValueSerializer;

    impl Serializer for ValueSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;
        type SerializeMap = MapSerializer;
        type SerializeSeq = SequenceSerializer;
        type SerializeStruct = MapSerializer;
        type SerializeStructVariant = MapSerializer;
        type SerializeTuple = SequenceSerializer;
        type SerializeTupleStruct = SequenceSerializer;
        type SerializeTupleVariant = SequenceSerializer;

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Bool(v))
        }

        fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::from(v)))
        }

        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Float(f64::from(v)))
        }

        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Float(v))
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Char(v))
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::String(Cow::Owned(v.to_string())))
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Bytes(Cow::Owned(v.to_vec())))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Identifier(Cow::Borrowed("None")))
        }

        fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            Ok(Value::Named(Named {
                name: Cow::Borrowed("Some"),
                contents: StructContents::Tuple(List(vec![value.serialize(ValueSerializer)?])),
            }))
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Tuple(List::default()))
        }

        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Identifier(Cow::Owned(name.to_string())))
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Identifier(Cow::Owned(variant.to_string())))
        }

        fn serialize_newtype_struct<T>(
            self,
            name: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            Ok(Value::Named(Named {
                name: Cow::Owned(name.to_string()),
                contents: StructContents::Tuple(List(vec![value.serialize(ValueSerializer)?])),
            }))
        }

        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            Ok(Value::Named(Named {
                name: Cow::Owned(variant.to_string()),
                contents: StructContents::Tuple(List(vec![value.serialize(ValueSerializer)?])),
            }))
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(SequenceSerializer::new(None, Nested::List))
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(SequenceSerializer::new(None, Nested::Tuple))
        }

        fn serialize_tuple_struct(
            self,
            name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(SequenceSerializer::new(
                Some(name.to_string()),
                Nested::Tuple,
            ))
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(SequenceSerializer::new(
                Some(variant.to_string()),
                Nested::Tuple,
            ))
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(MapSerializer {
                name: None,
                contents: Map::default(),
            })
        }

        fn serialize_struct(
            self,
            name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Ok(MapSerializer {
                name: Some(name.to_string()),
                contents: Map::default(),
            })
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(MapSerializer {
                name: Some(variant.to_string()),
                contents: Map::default(),
            })
        }

        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::try_from(v)?))
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Integer(Integer::try_from(v)?))
        }
    }

    pub struct SequenceSerializer {
        name: Option<String>,
        kind: Nested,
        contents: List<'static>,
    }

    impl SequenceSerializer {
        pub fn new(name: Option<String>, kind: Nested) -> Self {
            Self {
                name,
                kind,
                contents: List::default(),
            }
        }
    }

    impl SerializeSeq for SequenceSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            self.contents.0.push(value.serialize(ValueSerializer)?);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            match (self.name, self.kind) {
                (Some(name), Nested::List | Nested::Tuple) => Ok(Value::Named(Named {
                    name: Cow::Owned(name),
                    contents: StructContents::Tuple(self.contents),
                })),
                (None, Nested::List) => Ok(Value::Array(self.contents)),
                (None, Nested::Tuple) => Ok(Value::Tuple(self.contents)),
                (_, Nested::Map) => unreachable!(),
            }
        }
    }

    impl SerializeTuple for SequenceSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            SerializeSeq::end(self)
        }
    }

    impl SerializeTupleStruct for SequenceSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            SerializeSeq::end(self)
        }
    }

    impl SerializeTupleVariant for SequenceSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            SerializeSeq::end(self)
        }
    }

    #[derive(Default)]
    pub struct MapSerializer {
        name: Option<String>,
        contents: Map<'static>,
    }

    impl SerializeMap for MapSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            self.contents.0.push((
                key.serialize(ValueSerializer)?,
                Value::Tuple(List::default()),
            ));
            Ok(())
        }

        fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            self.contents
                .0
                .last_mut()
                .expect("serialize_key not called")
                .1 = value.serialize(ValueSerializer)?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            if let Some(name) = self.name {
                Ok(Value::Named(Named {
                    name: Cow::Owned(name),
                    contents: StructContents::Map(self.contents),
                }))
            } else {
                Ok(Value::Map(self.contents))
            }
        }
    }

    impl SerializeStruct for MapSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            SerializeMap::serialize_key(self, key)?;
            SerializeMap::serialize_value(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            SerializeMap::end(self)
        }
    }

    impl SerializeStructVariant for MapSerializer {
        type Error = ToValueError;
        type Ok = OwnedValue;

        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            SerializeMap::serialize_key(self, key)?;
            SerializeMap::serialize_value(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            SerializeMap::end(self)
        }
    }

    #[derive(Clone)]
    pub struct ValueDeserializer<'a, 'de>(pub &'a Value<'de>);

    macro_rules! deserialize_int {
        ($deserialize_name:ident, $as_name:ident, $visit_name:ident, $expected:expr) => {
            fn $deserialize_name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                if let Some(value) = self.0.$as_name() {
                    visitor.$visit_name(value)
                } else {
                    Err(FromValueError::Expected($expected))
                }
            }
        };
    }

    impl<'a, 'de> Deserializer<'de> for ValueDeserializer<'a, 'de> {
        type Error = FromValueError;

        deserialize_int!(deserialize_i8, as_i8, visit_i8, ExpectedKind::I8);

        deserialize_int!(deserialize_i16, as_i16, visit_i16, ExpectedKind::I16);

        deserialize_int!(deserialize_i32, as_i32, visit_i32, ExpectedKind::I32);

        deserialize_int!(deserialize_i64, as_i64, visit_i64, ExpectedKind::I64);

        deserialize_int!(deserialize_i128, as_i128, visit_i128, ExpectedKind::I128);

        deserialize_int!(deserialize_u8, as_u8, visit_u8, ExpectedKind::U8);

        deserialize_int!(deserialize_u16, as_u16, visit_u16, ExpectedKind::U16);

        deserialize_int!(deserialize_u32, as_u32, visit_u32, ExpectedKind::U32);

        deserialize_int!(deserialize_u64, as_u64, visit_u64, ExpectedKind::U64);

        deserialize_int!(deserialize_u128, as_u128, visit_u128, ExpectedKind::U128);

        deserialize_int!(deserialize_f64, as_f64, visit_f64, ExpectedKind::Float);

        #[allow(clippy::cast_possible_truncation)]
        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match &self.0 {
                Value::Integer(value) => match *value {
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
                Value::Float(value) => visitor.visit_f64(*value),
                Value::Bool(value) => visitor.visit_bool(*value),
                Value::Char(value) => visitor.visit_char(*value),
                Value::Byte(value) => visitor.visit_u8(*value),
                Value::Identifier(value) => match value {
                    Cow::Borrowed(str) => visitor.visit_borrowed_str(str),
                    Cow::Owned(str) => visitor.visit_str(str),
                },
                Value::String(value) => visitor.visit_str(value),
                Value::Bytes(value) => match value {
                    Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
                    Cow::Owned(bytes) => visitor.visit_bytes(bytes),
                },
                Value::Named(value) => match &value.contents {
                    StructContents::Map(map) => visitor.visit_map(MapDeserializer::new(map)),
                    StructContents::Tuple(list) => {
                        visitor.visit_seq(SequenceDeserializer(list.0.iter()))
                    }
                },
                Value::Array(list) | Value::Tuple(list) => {
                    visitor.visit_seq(SequenceDeserializer(list.0.iter()))
                }
                Value::Map(map) => visitor.visit_map(MapDeserializer::new(map)),
            }
        }

        fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match &self.0 {
                Value::Integer(int) => visitor.visit_bool(!int.is_zero()),
                Value::Bool(bool) => visitor.visit_bool(*bool),
                _ => Err(FromValueError::Expected(ExpectedKind::Bool)),
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            if let Some(value) = self.0.as_f64() {
                visitor.visit_f32(value as f32)
            } else {
                Err(FromValueError::Expected(ExpectedKind::Float))
            }
        }

        fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            if let Value::Char(ch) = &self.0 {
                visitor.visit_char(*ch)
            } else {
                Err(FromValueError::Expected(ExpectedKind::Char))
            }
        }

        fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match &self.0 {
                Value::String(str) | Value::Identifier(str) => match str {
                    Cow::Borrowed(str) => visitor.visit_borrowed_str(str),
                    Cow::Owned(str) => visitor.visit_str(str),
                },
                Value::Bytes(bytes) => match bytes {
                    Cow::Borrowed(bytes) => {
                        let str = str::from_utf8(bytes)?;
                        visitor.visit_borrowed_str(str)
                    }
                    Cow::Owned(bytes) => {
                        let str = str::from_utf8(bytes)?;
                        visitor.visit_str(str)
                    }
                },
                Value::Named(name) => match &name.name {
                    Cow::Borrowed(str) => visitor.visit_borrowed_str(str),
                    Cow::Owned(str) => visitor.visit_str(str),
                },
                _ => Err(FromValueError::Expected(ExpectedKind::String)),
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
            match &self.0 {
                Value::Bytes(bytes) => match bytes {
                    Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
                    Cow::Owned(bytes) => visitor.visit_bytes(bytes),
                },
                Value::String(str) | Value::Identifier(str) => match str {
                    Cow::Borrowed(str) => visitor.visit_borrowed_bytes(str.as_bytes()),
                    Cow::Owned(str) => visitor.visit_bytes(str.as_bytes()),
                },
                _ => Err(FromValueError::Expected(ExpectedKind::Bytes)),
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
            match &self.0 {
                Value::String(name) | Value::Identifier(name) if name == "None" => {
                    visitor.visit_none()
                }
                Value::Named(Named {
                    name,
                    contents: StructContents::Tuple(list),
                }) if name == "Some" && !list.0.is_empty() => {
                    visitor.visit_some(ValueDeserializer(list.0.first().expect("length checked")))
                }
                // To support changing a T to Option<T>, we can fuzzily allow a
                // value to be treated as Some(). TODO should this be optional?
                _ => visitor.visit_some(self),
            }
        }

        fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match &self.0 {
                Value::Named(Named {
                    contents: StructContents::Tuple(_),
                    ..
                })
                | Value::Tuple(_)
                | Value::Array(_) => visitor.visit_unit(),
                _ => Err(FromValueError::Expected(ExpectedKind::Unit)),
            }
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
            match &self.0 {
                Value::Named(named) if named.name == name => match &named.contents {
                    StructContents::Tuple(contents) => {
                        if let Some(first) = contents.0.first() {
                            visitor.visit_newtype_struct(ValueDeserializer(first))
                        } else {
                            visitor.visit_newtype_struct(ValueDeserializer(&Value::unit()))
                        }
                    }
                    StructContents::Map(map) => {
                        if let Some((_, first)) = map.0.first() {
                            visitor.visit_newtype_struct(ValueDeserializer(first))
                        } else {
                            visitor.visit_newtype_struct(ValueDeserializer(&Value::unit()))
                        }
                    }
                },
                _ => visitor.visit_newtype_struct(self),
            }
        }

        fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match &self.0 {
                Value::Named(Named {
                    contents: StructContents::Tuple(list),
                    ..
                })
                | Value::Tuple(list)
                | Value::Array(list) => visitor.visit_seq(SequenceDeserializer(list.0.iter())),
                _ => Err(FromValueError::Expected(ExpectedKind::Sequence)),
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
            _name: &'static str,
            len: usize,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            self.deserialize_tuple(len, visitor)
        }

        fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match &self.0 {
                Value::Named(Named {
                    contents: StructContents::Map(map),
                    ..
                })
                | Value::Map(map) => visitor.visit_map(MapDeserializer::new(map)),
                _ => Err(FromValueError::Expected(ExpectedKind::Map)),
            }
        }

        fn deserialize_struct<V>(
            self,
            _name: &'static str,
            _fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            self.deserialize_map(visitor)
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
            self.deserialize_any(visitor)
        }
    }

    impl<'a, 'de> EnumAccess<'de> for ValueDeserializer<'a, 'de> {
        type Error = FromValueError;
        type Variant = EnumVariantAccessor<'a, 'de>;

        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
        where
            V: serde::de::DeserializeSeed<'de>,
        {
            match &self.0 {
                Value::Identifier(_) | Value::String(_) => {
                    Ok((seed.deserialize(self)?, EnumVariantAccessor::Unit))
                }
                Value::Named(named) => {
                    let variant =
                        seed.deserialize(ValueDeserializer(&Value::String(named.name.clone())))?;

                    let accessor = match &named.contents {
                        StructContents::Map(map) => EnumVariantAccessor::Map(map),
                        StructContents::Tuple(list) => EnumVariantAccessor::Tuple(list),
                    };

                    Ok((variant, accessor))
                }
                _ => Err(FromValueError::Expected(ExpectedKind::Enum)),
            }
        }
    }

    pub enum EnumVariantAccessor<'a, 'de> {
        Unit,
        Tuple(&'a List<'de>),
        Map(&'a Map<'de>),
    }

    impl<'a, 'de> VariantAccess<'de> for EnumVariantAccessor<'a, 'de> {
        type Error = FromValueError;

        fn unit_variant(self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: serde::de::DeserializeSeed<'de>,
        {
            match self {
                EnumVariantAccessor::Unit => seed.deserialize(ValueDeserializer(&Value::unit())),
                EnumVariantAccessor::Tuple(list) => {
                    if let Some(first) = list.0.first() {
                        seed.deserialize(ValueDeserializer(first))
                    } else {
                        seed.deserialize(ValueDeserializer(&Value::unit()))
                    }
                }
                EnumVariantAccessor::Map(_) => Err(FromValueError::Expected(ExpectedKind::Newtype)),
            }
        }

        fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match self {
                EnumVariantAccessor::Unit => visitor.visit_seq(SequenceDeserializer([].iter())),
                EnumVariantAccessor::Tuple(list) => {
                    visitor.visit_seq(SequenceDeserializer(list.0.iter()))
                }
                EnumVariantAccessor::Map(map) => visitor.visit_map(MapDeserializer::new(map)),
            }
        }

        fn struct_variant<V>(
            self,
            _fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match self {
                EnumVariantAccessor::Unit => visitor.visit_seq(SequenceDeserializer([].iter())),
                EnumVariantAccessor::Tuple(list) => {
                    visitor.visit_seq(SequenceDeserializer(list.0.iter()))
                }
                EnumVariantAccessor::Map(map) => visitor.visit_map(MapDeserializer::new(map)),
            }
        }
    }

    pub struct MapDeserializer<'a, 'de> {
        map: &'a Map<'de>,
        index: usize,
    }

    impl<'a, 'de> MapDeserializer<'a, 'de> {
        pub fn new(map: &'a Map<'de>) -> Self {
            Self { map, index: 0 }
        }
    }

    impl<'a, 'de> MapAccess<'de> for MapDeserializer<'a, 'de> {
        type Error = FromValueError;

        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: serde::de::DeserializeSeed<'de>,
        {
            self.map
                .0
                .get(self.index)
                .map(|item| seed.deserialize(ValueDeserializer(&item.0)))
                .transpose()
        }

        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::DeserializeSeed<'de>,
        {
            let value = seed.deserialize(ValueDeserializer(&self.map.0[self.index].1))?;
            self.index += 1;
            Ok(value)
        }
    }

    pub struct SequenceDeserializer<'a, 'de>(slice::Iter<'a, Value<'de>>);

    impl<'a, 'de> SeqAccess<'de> for SequenceDeserializer<'a, 'de> {
        type Error = FromValueError;

        fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where
            T: serde::de::DeserializeSeed<'de>,
        {
            self.0
                .next()
                .map(|item| seed.deserialize(ValueDeserializer(item)))
                .transpose()
        }
    }

    /// An error from serializing to a [`Value`].
    #[derive(Debug, PartialEq)]

    pub enum ToValueError {
        /// A custom serialization error.
        Message(String),
        /// An integer was too larget to represent.
        IntegerTooLarge(TryFromIntError),
    }

    impl From<TryFromIntError> for ToValueError {
        fn from(value: TryFromIntError) -> Self {
            Self::IntegerTooLarge(value)
        }
    }

    impl serde::ser::Error for ToValueError {
        fn custom<T>(msg: T) -> Self
        where
            T: Display,
        {
            Self::Message(msg.to_string())
        }
    }

    impl serde::ser::StdError for ToValueError {}

    impl Display for ToValueError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                ToValueError::Message(message) => f.write_str(message),
                ToValueError::IntegerTooLarge(err) => write!(f, "{err}"),
            }
        }
    }

    /// An error from deserializing from a [`Value`].
    #[derive(Debug, PartialEq)]

    pub enum FromValueError {
        /// A custom serialization error.
        Message(String),
        /// Expected a kind of data, but encountered another kind.
        Expected(ExpectedKind),
        /// Invalid UTF-8 was encountered.
        InvalidUtf8,
    }

    impl serde::de::Error for FromValueError {
        fn custom<T>(msg: T) -> Self
        where
            T: Display,
        {
            Self::Message(msg.to_string())
        }
    }

    impl From<Utf8Error> for FromValueError {
        fn from(_value: Utf8Error) -> Self {
            Self::InvalidUtf8
        }
    }

    impl serde::de::StdError for FromValueError {}

    impl Display for FromValueError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                FromValueError::Message(message) => f.write_str(message),
                FromValueError::Expected(kind) => write!(f, "expected {kind}"),
                FromValueError::InvalidUtf8 => {
                    f.write_str("invalid utf8 when interpreting bytes as a string")
                }
            }
        }
    }

    #[derive(Debug, PartialEq)]
    pub enum ExpectedKind {
        I8,
        I16,
        I32,
        I64,
        I128,
        U8,
        U16,
        U32,
        U64,
        U128,
        Bool,
        Char,
        String,
        Bytes,
        Float,
        Enum,
        Map,
        Sequence,
        Unit,
        Newtype,
    }

    impl Display for ExpectedKind {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                ExpectedKind::I8 => f.write_str("i8"),
                ExpectedKind::I16 => f.write_str("i16"),
                ExpectedKind::I32 => f.write_str("i32"),
                ExpectedKind::I64 => f.write_str("i64"),
                ExpectedKind::I128 => f.write_str("i128"),
                ExpectedKind::U8 => f.write_str("u8"),
                ExpectedKind::U16 => f.write_str("u16"),
                ExpectedKind::U32 => f.write_str("u32"),
                ExpectedKind::U64 => f.write_str("u64"),
                ExpectedKind::U128 => f.write_str("u128"),
                ExpectedKind::Bool => f.write_str("bool"),
                ExpectedKind::Char => f.write_str("char"),
                ExpectedKind::String => f.write_str("String"),
                ExpectedKind::Bytes => f.write_str("Bytes"),
                ExpectedKind::Float => f.write_str("Float"),
                ExpectedKind::Enum => f.write_str("Enum"),
                ExpectedKind::Map => f.write_str("Map"),
                ExpectedKind::Sequence => f.write_str("Sequence"),
                ExpectedKind::Unit => f.write_str("()"),
                ExpectedKind::Newtype => f.write_str("Newtype"),
            }
        }
    }
}

#[test]
fn display() {
    use alloc::string::ToString;
    use alloc::{format, vec};

    assert_eq!(
        Value::Named(Named {
            name: Cow::Borrowed("Hello"),
            contents: StructContents::Tuple(List(vec![Value::String(Cow::Borrowed("World"))]))
        })
        .to_string(),
        "Hello(\"World\")"
    );

    assert_eq!(
        format!(
            "{:#}",
            Value::Named(Named {
                name: Cow::Borrowed("Hello"),
                contents: StructContents::Tuple(List(vec![Value::String(Cow::Borrowed("World"))]))
            })
        ),
        "Hello(\n  \"World\"\n)"
    );
}

#[cfg(feature = "serde")]
pub use self::serde::{FromValueError, ToValueError};
