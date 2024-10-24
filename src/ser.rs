use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt::Write;

use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde::Serialize;

use crate::writer::{self, Writer};

/// A Serde serializer that generates Rsn.
#[derive(Debug)]
pub struct Serializer<'config, Output> {
    writer: Writer<'config, Output>,
    implicit_map_at_root: bool,
    anonymous_structs: bool,
}

impl Default for Serializer<'static, String> {
    fn default() -> Self {
        Self {
            writer: Writer::default(),
            implicit_map_at_root: false,
            anonymous_structs: false,
        }
    }
}

impl<'config, Output> Serializer<'config, Output>
where
    Output: Write,
{
    /// Returns a new serializer that writes to `output` using `configuration`.
    pub fn new(output: Output, configuration: &'config Config) -> Self {
        Self {
            writer: Writer::new(output, &configuration.writer),
            implicit_map_at_root: configuration.implicit_map_at_root,
            anonymous_structs: configuration.anonymous_structs,
        }
    }

    /// Finishes writing to the output and returns the output.
    pub fn finish(self) -> Output {
        self.writer.finish()
    }

    fn mark_value_seen(&mut self) {
        self.implicit_map_at_root = false;
    }
}

impl<'a, 'config, Output> serde::Serializer for &'a mut Serializer<'config, Output>
where
    Output: Write,
{
    type Error = core::fmt::Error;
    type Ok = ();
    type SerializeMap = sealed::MapSerializer<'a, 'config, Output>;
    type SerializeSeq = Self;
    type SerializeStruct = sealed::MapSerializer<'a, 'config, Output>;
    type SerializeStructVariant = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(&v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_primitive(v)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value("None")
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        self.mark_value_seen();
        self.writer.begin_named_tuple("Some")?;
        value.serialize(&mut *self)?;
        self.writer.finish_nested()?;
        Ok(())
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value("()")
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value(name)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.mark_value_seen();
        self.writer.write_raw_value(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        self.mark_value_seen();
        self.writer.begin_named_tuple(name)?;
        value.serialize(&mut *self)?;
        self.writer.finish_nested()
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
        self.mark_value_seen();
        self.writer.begin_named_tuple(variant)?;
        value.serialize(&mut *self)?;
        self.writer.finish_nested()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_list()?;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_tuple()?;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.mark_value_seen();
        self.writer.begin_named_tuple(name)?;
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
        self.writer.begin_named_tuple(variant)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let is_implicit_map = self.implicit_map_at_root;
        self.mark_value_seen();
        if !is_implicit_map {
            self.writer.begin_map()?;
        }
        Ok(sealed::MapSerializer {
            serializer: self,
            is_implicit_map,
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let is_implicit_map = self.implicit_map_at_root;
        self.mark_value_seen();

        if !is_implicit_map {
            if self.anonymous_structs {
                self.writer.begin_map()?;
            } else {
                self.writer.begin_named_map(name)?;
            }
        }

        Ok(sealed::MapSerializer {
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
        self.writer.begin_named_map(variant)?;
        Ok(self)
    }
}

impl<'a, 'config, Output> SerializeSeq for &'a mut Serializer<'config, Output>
where
    Output: Write,
{
    type Error = core::fmt::Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested()
    }
}

impl<'a, 'config, Output> SerializeTuple for &'a mut Serializer<'config, Output>
where
    Output: Write,
{
    type Error = core::fmt::Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested()
    }
}

impl<'a, 'config, Output> SerializeTupleStruct for &'a mut Serializer<'config, Output>
where
    Output: Write,
{
    type Error = core::fmt::Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested()
    }
}

impl<'a, 'config, Output> SerializeTupleVariant for &'a mut Serializer<'config, Output>
where
    Output: Write,
{
    type Error = core::fmt::Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.finish_nested()
    }
}

mod sealed {
    use super::{SerializeMap, SerializeStruct, SerializeStructVariant, Serializer, Write};

    pub struct MapSerializer<'a, 'config, Output> {
        pub serializer: &'a mut Serializer<'config, Output>,
        pub is_implicit_map: bool,
    }

    impl<'a, 'config, Output> SerializeStruct for MapSerializer<'a, 'config, Output>
    where
        Output: Write,
    {
        type Error = core::fmt::Error;
        type Ok = ();

        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            if self.is_implicit_map {
                self.serializer.writer.write_raw_value(key)?;
                self.serializer.writer.write_raw_value(": ")?;
                value.serialize(&mut *self.serializer)?;
                self.serializer.writer.insert_newline()?;
            } else {
                self.serializer.writer.write_raw_value(key)?;
                value.serialize(&mut *self.serializer)?;
            }
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            if !self.is_implicit_map {
                self.serializer.writer.finish_nested()?;
            }
            Ok(())
        }
    }

    impl<'a, 'config, Output> SerializeStructVariant for &'a mut Serializer<'config, Output>
    where
        Output: Write,
    {
        type Error = core::fmt::Error;
        type Ok = ();

        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            self.writer.write_raw_value(key)?;
            value.serialize(&mut **self)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.finish_nested()
        }
    }

    impl<'a, 'config, Output> SerializeMap for MapSerializer<'a, 'config, Output>
    where
        Output: Write,
    {
        type Error = core::fmt::Error;
        type Ok = ();

        fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            key.serialize(&mut *self.serializer)
        }

        fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize + ?Sized,
        {
            if self.is_implicit_map {
                self.serializer.writer.write_raw_value(": ")?;
                value.serialize(&mut *self.serializer)?;
                self.serializer.writer.insert_newline()
            } else {
                value.serialize(&mut *self.serializer)
            }
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            if !self.is_implicit_map {
                self.serializer.writer.finish_nested()?;
            }
            Ok(())
        }
    }
}

/// The configuration for a [`Serializer`].
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    /// The writer configuration.
    pub writer: writer::Config,
    /// Whether a map-like type at the root of the document should use the
    /// implicit map syntax.
    pub implicit_map_at_root: bool,
    /// Whether to include the names of structures in.
    pub anonymous_structs: bool,
}

impl Config {
    /// Returns the default configuration.
    ///
    /// The default configuration uses:
    ///
    /// - `writer`: [`writer::Config::Compact`]
    /// - `implicit_map_at_root`: `false`
    /// - `anonymous_structs`: `false`
    #[must_use]
    pub const fn new() -> Self {
        Self {
            writer: writer::Config::Compact,
            implicit_map_at_root: false,
            anonymous_structs: false,
        }
    }

    /// Returns the default configuration with nested indentation and newlines.
    #[must_use]
    pub fn pretty() -> Self {
        Self {
            writer: writer::Config::Pretty {
                indentation: Cow::Borrowed("  "),
                newline: Cow::Borrowed("\n"),
            },
            ..Default::default()
        }
    }

    /// Sets [`Config::implicit_map_at_root`] and returns self.
    #[must_use]
    pub const fn implicit_map_at_root(mut self, implicit_map_at_root: bool) -> Self {
        self.implicit_map_at_root = implicit_map_at_root;
        self
    }

    /// Sets [`Config::anonymous_structs`] and returns self.
    #[must_use]
    pub const fn anonymous_structs(mut self, anonymous_structs: bool) -> Self {
        self.anonymous_structs = anonymous_structs;
        self
    }

    /// Returns `value` serialized as Rsn with this configuration.
    ///
    /// # Errors
    ///
    /// Rsn itself does not produce any errors while serializing values. This
    /// function will return errors that arise within `Serialize` implementations
    /// encountered while serializing `value`.
    pub fn serialize<S: Serialize>(&self, value: &S) -> Result<String, core::fmt::Error> {
        let mut serializer = Serializer::new(String::new(), self);
        value.serialize(&mut serializer)?;
        Ok(serializer.finish())
    }

    /// Returns `value` serialized as Rsn with this configuration.
    ///
    /// # Errors
    ///
    /// Rsn itself does not produce any errors while serializing values. This
    /// function will return errors that arise within `Serialize` implementations
    /// encountered while serializing `value`.
    pub fn serialize_to_vec<S: Serialize>(
        &self,
        value: &S,
    ) -> Result<alloc::vec::Vec<u8>, core::fmt::Error> {
        self.serialize(value).map(String::into_bytes)
    }
}

#[cfg(feature = "std")]
mod serialize_writer {
    use super::{Config, Serialize, Serializer, Write};

    struct Writer<T> {
        inner: T,
        written: usize,
        error: Option<std::io::Error>,
    }
    impl<T: std::io::Write> Write for Writer<T> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            match self.inner.write(s.as_bytes()) {
                Ok(written) => {
                    self.written += written;
                    Ok(())
                }
                Err(error) => {
                    assert!(
                        self.error.is_none(),
                        "should not have continued on write error"
                    );
                    self.error = Some(error);
                    Err(core::fmt::Error)
                }
            }
        }
    }
    impl Config {
        /// Serializes `value` into `writer` using this configuration.
        ///
        /// # Errors
        ///
        /// Returns any errors occurring while serializing `value` or while
        /// writing to `writer`.
        #[allow(clippy::missing_panics_doc)]
        pub fn serialize_to_writer<S: Serialize, W: std::io::Write>(
            &self,
            value: &S,
            writer: W,
        ) -> std::io::Result<usize> {
            let mut writer = Writer {
                inner: writer,
                written: 0,
                error: None,
            };
            let mut serializer = Serializer::new(&mut writer, self);
            value
                .serialize(&mut serializer)
                .map_err(|_| writer.error.expect("should store error on error"))?;
            Ok(writer.written)
        }
    }
}

#[test]
fn serialization_test() {
    #[derive(Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
    struct BasicNamed {
        a: u32,
        b: i32,
    }

    let rendered = crate::to_string(&BasicNamed { a: 1, b: -1 }).expect("no errors");
    assert_eq!(rendered, "BasicNamed{a:1,b:-1}");
}
