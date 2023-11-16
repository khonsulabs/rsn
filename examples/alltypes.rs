use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Serialize, Deserialize, Debug)]
struct ExampleStruct {
    integers: Vec<usize>,
    floats: Vec<f64>,
    bools: Vec<bool>,
    chars: Vec<char>,
    string: String,
    raw_string: String,
    bytes: Vec<u8>,
    byte_string: ByteBuf,
    raw_byte_string: ByteBuf,
    named_map: NamedExample,
    named_tuple: NamedExample,
    r#raw_identifiers: bool,
    array: Vec<usize>,
    tuple: Vec<usize>,
    map: HashMap<String, usize>,
}

#[derive(Serialize, Deserialize, Debug)]
enum NamedExample {
    StructLike { field: usize },
    TupleLike(usize),
}

fn main() {
    let example: ExampleStruct =
        rsn::from_str(include_str!("./alltypes.rsn")).expect("error deserializing alltypes.rsn");

    println!("Loaded blog posts: {example:?}");
}

#[test]
fn runs() {
    main();
}
