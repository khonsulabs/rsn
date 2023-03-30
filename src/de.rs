use alloc::borrow::Cow;
use alloc::string::String;
use core::iter::Peekable;

use serde::de::{MapAccess, SeqAccess};

use crate::parser::{Error, Event, Nested, Parser, Primitive};

pub struct Deserializer<'de> {
    parser: Peekable<Parser<'de>>,
}

impl<'de> Deserializer<'de> {
    pub fn new(source: &'de str) -> Self {
        Self {
            parser: Parser::new(source).peekable(),
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
                Some(Event::Primitive(Primitive::Integer(value))) => {
                    visitor.$visit_name(value.$conv_name().unwrap())
                }
                Some(_) => todo!("expected integer"),
                None => todo!("unexpected eof"),
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

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event::Primitive(Primitive::Identifier(str)))
            | Some(Event::Primitive(Primitive::String(str))) => match str {
                Cow::Borrowed(str) => visitor.visit_borrowed_str(str),
                Cow::Owned(str) => visitor.visit_string(str),
            },
            Some(Event::Primitive(Primitive::Bytes(bytes))) => match bytes {
                Cow::Borrowed(bytes) => {
                    visitor.visit_borrowed_str(core::str::from_utf8(bytes).unwrap())
                }
                Cow::Owned(bytes) => visitor.visit_string(String::from_utf8(bytes).unwrap()),
            },
            Some(_) => todo!("expected identifier"),
            None => todo!("unexpected eof"),
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
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event::BeginNested { kind, .. }) => {
                if !matches!(kind, Nested::Tuple | Nested::List) {
                    todo!("expected a tuple")
                }

                visitor.visit_seq(self)
            }
            Some(other) => {
                todo!("expected struct")
            }
            None => todo!("unexpected eof"),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event::BeginNested { kind, .. }) => {
                if kind != Nested::Tuple {
                    todo!("expected a tuple")
                }

                visitor.visit_seq(self)
            }
            Some(other) => {
                todo!("expected struct")
            }
            None => todo!("unexpected eof"),
        }
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
            Some(Event::BeginNested { name, kind }) => {
                if name.map_or(false, |name| name != struct_name) {
                    todo!("struct name mismatch")
                }

                if kind != Nested::Tuple {
                    todo!("expected a tuple")
                }

                visitor.visit_seq(self)
            }
            Some(other) => {
                todo!("expected struct")
            }
            None => todo!("unexpected eof"),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parser.next().transpose()? {
            Some(Event::BeginNested { kind, .. }) => {
                if kind != Nested::Map {
                    todo!("expected a map")
                }

                visitor.visit_map(self)
            }
            Some(other) => {
                todo!("expected struct")
            }
            None => todo!("unexpected eof"),
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
            Some(Event::BeginNested { name, kind }) => {
                if name.map_or(false, |name| name != struct_name) {
                    todo!("struct name mismatch")
                }

                if kind != Nested::Map {
                    todo!("expected a map")
                }

                visitor.visit_map(self)
            }
            Some(other) => {
                todo!("expected struct")
            }
            None => todo!("unexpected eof"),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
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
        todo!()
    }
}

impl<'de> MapAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.parser.peek() {
            Some(Ok(Event::EndNested)) => {
                self.parser.next();
                Ok(None)
            }
            Some(_) => seed.deserialize(&mut *self).map(Some),
            None => todo!("unexpected eof"),
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
            Some(Ok(Event::EndNested)) => {
                self.parser.next();
                Ok(None)
            }
            Some(_) => seed.deserialize(&mut *self).map(Some),
            None => todo!("unexpected eof"),
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        todo!("custom error: {msg}")
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

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
}
