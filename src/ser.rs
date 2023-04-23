use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt::Display;
use serde::Serialize;

use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::writer::{self, Writer};

#[derive(Debug, Default)]
pub struct Serializer<'config> {
    writer: Writer<'config>,
    implicit_map_at_root: bool,
}

impl<'config> Serializer<'config> {
    pub fn new(config: &'config Config) -> Self {
        Self {
            writer: Writer::new(&config.writer),
            implicit_map_at_root: config.implicit_map_at_root,
        }
    }

    pub fn finish(self) -> String {
        self.writer.finish()
    }

    fn mark_value_seen(&mut self) {
        self.implicit_map_at_root = false;
    }
}

impl<'a, 'config> serde::Serializer for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();
    type SerializeMap = Self;
    type SerializeSeq = Self;
    type SerializeStruct = StructSerializer<'a, 'config>;
    type SerializeStructVariant = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value("None");
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.mark_value_seen();
        self.writer.begin_named_tuple("Some");
        value.serialize(&mut *self)?;
        self.writer.finish_nested();
        Ok(())
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value("()");
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value(name);
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value(variant);
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.mark_value_seen();
        self.writer.begin_named_tuple(name);
        value.serialize(&mut *self)?;
        self.writer.finish_nested();
        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.mark_value_seen();
        self.writer.begin_named_tuple(variant);
        value.serialize(&mut *self)?;
        self.writer.finish_nested();
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_list();
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_tuple();
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_named_tuple(name);
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_named_tuple(variant);
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_map();
        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let is_implicit_map = self.implicit_map_at_root;
        self.mark_value_seen();

        if !is_implicit_map {
            self.writer.begin_named_map(name);
        }

        Ok(StructSerializer {
            serializer: self,
            is_implicit_map,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.writer.begin_named_map(variant);
        Ok(self)
    }
}

impl<'a, 'config> SerializeSeq for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested();
        Ok(())
    }
}

impl<'a, 'config> SerializeTuple for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested();
        Ok(())
    }
}

impl<'a, 'config> SerializeTupleStruct for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested();
        Ok(())
    }
}

impl<'a, 'config> SerializeTupleVariant for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested();
        Ok(())
    }
}

pub struct StructSerializer<'a, 'config> {
    serializer: &'a mut Serializer<'config>,
    is_implicit_map: bool,
}

impl<'a, 'config> SerializeStruct for StructSerializer<'a, 'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if self.is_implicit_map {
            self.serializer.writer.write_raw_value(key);
            self.serializer.writer.write_raw_value(": ");
            value.serialize(&mut *self.serializer)?;
            self.serializer.writer.insert_newline();
        } else {
            self.serializer.writer.write_raw_value(key);
            value.serialize(&mut *self.serializer)?;
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.is_implicit_map {
            self.serializer.writer.finish_nested();
        }
        Ok(())
    }
}

impl<'a, 'config> SerializeStructVariant for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.writer.write_raw_value(key);
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested();
        Ok(())
    }
}

impl<'a, 'config> SerializeMap for &'a mut Serializer<'config> {
    type Error = Infallible;
    type Ok = ();

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested();
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct Config {
    pub writer: writer::Config,
    pub implicit_map_at_root: bool,
}

impl Config {
    pub fn pretty() -> Self {
        Self {
            writer: writer::Config::Pretty {
                indentation: Cow::Borrowed("  "),
                newline: Cow::Borrowed("\n"),
            },
            implicit_map_at_root: false,
        }
    }

    pub const fn implicit_map_at_root(mut self, implicit_map_at_root: bool) -> Self {
        self.implicit_map_at_root = implicit_map_at_root;
        self
    }

    pub fn serialize<S: Serialize>(&self, value: &S) -> String {
        let mut serializer = Serializer::new(self);
        value.serialize(&mut serializer).expect("infallible");
        serializer.finish()
    }
}

#[derive(Debug)]
pub enum Infallible {}

impl serde::ser::Error for Infallible {
    fn custom<T>(_msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        unreachable!("rsn is infallible when serializing")
    }
}

impl Display for Infallible {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unreachable!("this type cannot be constructed")
    }
}

impl serde::ser::StdError for Infallible {}

#[test]
fn serialization_test() {
    #[derive(Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
    struct BasicNamed {
        a: u32,
        b: i32,
    }

    let rendered = crate::to_string(&BasicNamed { a: 1, b: -1 });
    assert_eq!(rendered, r#"BasicNamed{a:1,b:-1}"#);
}
