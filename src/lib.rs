#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "serde")]
pub mod de;
pub mod parser;
#[cfg(feature = "serde")]
pub mod ser;
pub mod tokenizer;
pub mod value;
pub mod writer;

#[cfg(feature = "serde")]
pub fn from_str<'de, D: serde::Deserialize<'de>>(source: &'de str) -> Result<D, de::Error> {
    parser::Config::default().deserialize(source)
}

#[cfg(feature = "serde")]
pub fn from_slice<'de, D: serde::Deserialize<'de>>(source: &'de [u8]) -> Result<D, de::Error> {
    parser::Config::default().deserialize_slice(source)
}

#[cfg(all(feature = "serde", feature = "std"))]
pub fn from_reader<D: serde::de::DeserializeOwned, R: std::io::Read>(
    reader: R,
) -> Result<D, de::Error> {
    parser::Config::default().deserialize_reader(reader)
}

#[cfg(feature = "serde")]
pub fn to_string<S: serde::Serialize>(value: &S) -> alloc::string::String {
    ser::Config::default().serialize(value)
}

#[cfg(feature = "serde")]
pub fn to_vec<S: serde::Serialize>(value: &S) -> alloc::vec::Vec<u8> {
    ser::Config::default().serialize_to_vec(value)
}

#[cfg(all(feature = "serde", feature = "std"))]
pub fn to_writer<S: serde::Serialize, W: std::io::Write>(
    value: &S,
    writer: W,
) -> std::io::Result<usize> {
    ser::Config::default().serialize_writer(value, writer)
}

#[cfg(feature = "serde")]
pub fn to_string_pretty<S: serde::Serialize>(value: &S) -> alloc::string::String {
    ser::Config::pretty().serialize(value)
}

#[cfg(test)]
mod tests;
