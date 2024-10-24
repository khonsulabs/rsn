#![no_std]
#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

/// Serde deserialization support.
#[cfg(feature = "serde")]
pub mod de;
/// Parse data or a reader into a sequence of Rsn events.
pub mod parser;
/// Serde serialization support.
#[cfg(feature = "serde")]
pub mod ser;
/// Parse data or a reader into a sequence of tokens.
pub mod tokenizer;
/// Types for generically representing the parsed value from an Rsn document.
pub mod value;
/// Types for writing Rsn.
pub mod writer;

/// Deserializes `D` from `source` using the default Rsn
/// [`Config`](parser::Config).
///
/// # Errors
///
/// Returns an error if `source` isn't valid Rsn or cannot be deserialized as
/// `D`.
#[cfg(feature = "serde")]
pub fn from_str<'de, D: serde::Deserialize<'de>>(source: &'de str) -> Result<D, de::Error> {
    parser::Config::default().deserialize(source)
}

/// Deserializes `D` from `slice` using the default Rsn
/// [`Config`](parser::Config).
///
/// # Errors
///
/// Returns an error if `slice` isn't valid Rsn or cannot be deserialized as
/// `D`.
#[cfg(feature = "serde")]
pub fn from_slice<'de, D: serde::Deserialize<'de>>(source: &'de [u8]) -> Result<D, de::Error> {
    parser::Config::default().deserialize_from_slice(source)
}

/// Deserializes `D` from `reader` using the default Rsn
/// [`Config`](parser::Config).
///
/// # Errors
///
/// Returns an error if `reader` returns an error while reading, doesn't contain
/// valid Rsn, or cannot be deserialized as `D`.
#[cfg(all(feature = "serde", feature = "std"))]
pub fn from_reader<D: serde::de::DeserializeOwned, R: std::io::Read>(
    reader: R,
) -> Result<D, de::Error> {
    parser::Config::default().deserialize_from_reader(reader)
}

/// Serializes `value` into a `String` using the default Rsn
/// [`Config`](ser::Config).
///
/// # Errors
///
/// Rsn itself does not produce any errors while serializing values. This
/// function will return errors that arise within `Serialize` implementations
/// encountered while serializing `value`.
#[cfg(feature = "serde")]
pub fn to_string<S: serde::Serialize>(
    value: &S,
) -> Result<alloc::string::String, core::fmt::Error> {
    ser::Config::default().serialize(value)
}

/// Serializes `value` into a `Vec<u8>` using the default Rsn
/// [`Config`](ser::Config).
///
/// # Errors
///
/// Rsn itself does not produce any errors while serializing values. This
/// function will return errors that arise within `Serialize` implementations
/// encountered while serializing `value`.
#[cfg(feature = "serde")]
pub fn to_vec<S: serde::Serialize>(value: &S) -> Result<alloc::vec::Vec<u8>, core::fmt::Error> {
    ser::Config::default().serialize_to_vec(value)
}

/// Serializes `value` into a writer using the default Rsn
/// [`Config`](ser::Config).
///
/// # Errors
///
/// Returns any errors occurring while serializing `value` or while writing to
/// `writer`.
#[cfg(all(feature = "serde", feature = "std"))]
pub fn to_writer<S: serde::Serialize, W: std::io::Write>(
    value: &S,
    writer: W,
) -> std::io::Result<usize> {
    ser::Config::default().serialize_to_writer(value, writer)
}

/// Serializes `value` into a `String` using
/// [`Config::pretty()`](ser::Config::pretty()).
///
/// # Errors
///
/// Rsn itself does not produce any errors while serializing values. This
/// function will return errors that arise within `Serialize` implementations
/// encountered while serializing `value`.
#[cfg(feature = "serde")]
pub fn to_string_pretty<S: serde::Serialize>(
    value: &S,
) -> Result<alloc::string::String, core::fmt::Error> {
    ser::Config::pretty().serialize(value)
}

#[cfg(test)]
mod tests;
