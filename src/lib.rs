#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "serde")]
pub mod de;
pub mod parser;
pub mod tokenizer;
pub mod value;

#[cfg(feature = "serde")]
pub fn from_str<'de, D: serde::Deserialize<'de>>(source: &'de str) -> Result<D, de::Error> {
    let mut parser = de::Deserializer::new(source, parser::Config::default());
    // TODO verify eof
    D::deserialize(&mut parser)
}

#[cfg(test)]
mod tests;
