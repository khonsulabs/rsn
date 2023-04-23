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
pub fn to_string<S: serde::Serialize>(value: &S) -> alloc::string::String {
    let mut serializer = ser::Serializer::default();
    value.serialize(&mut serializer).expect("infallible");
    serializer.finish()
}

#[cfg(feature = "serde")]
pub fn to_string_pretty<S: serde::Serialize>(value: &S) -> alloc::string::String {
    ser::Config::pretty().serialize(value)
}

#[cfg(test)]
mod tests;
