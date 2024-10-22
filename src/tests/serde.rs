use alloc::borrow::Cow;
use alloc::vec;
use core::fmt::Debug;
use std::collections::BTreeMap;
use std::string::String;

use serde::{Deserialize, Serialize};

use crate::value::Value;
use crate::{dbg, println};

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct StructOfEverything<'a> {
    str: Cow<'a, str>,
    bytes: serde_bytes::ByteBuf,
    char: char,
    u8: u8,
    u16: u16,
    u32: u32,
    u64: u64,
    u128: u128,
    usize: usize,
    i8: i8,
    i16: i16,
    i32: i32,
    i64: i64,
    i128: i128,
    isize: isize,
    bool: bool,
}

impl<'a> StructOfEverything<'a> {
    fn min() -> Self {
        Self {
            str: Cow::Borrowed("\0"),
            bytes: serde_bytes::ByteBuf::from(vec![0]),
            char: '\0',
            u8: 0,
            u16: 0,
            u32: 0,
            u64: 0,
            u128: 0,
            usize: 0,
            i8: i8::MIN,
            i16: i16::MIN,
            i32: i32::MIN,
            i64: i64::MIN,
            i128: i128::from(i64::MIN), /* To make deserialization strings consistent and compatible across feature flags */
            isize: isize::MIN,
            bool: false,
        }
    }

    fn max() -> Self {
        Self {
            str: Cow::Borrowed("hello \u{1_F980}"),
            bytes: serde_bytes::ByteBuf::from(b"hello, world".to_vec()),
            char: '\u{1_F980}',
            u8: u8::MAX,
            u16: u16::MAX,
            u32: u32::MAX,
            u64: u64::MAX,
            u128: u128::from(u64::MAX), /* To make deserialization strings consistent and compatible across feature flags */
            usize: usize::MAX,
            i8: i8::MAX,
            i16: i16::MAX,
            i32: i32::MAX,
            i64: i64::MAX,
            i128: i128::from(i64::MAX), /* To make deserialization strings consistent and compatible across feature flags */
            isize: isize::MAX,
            bool: true,
        }
    }
}

#[track_caller]
fn roundtrip<T: Debug + Serialize + for<'de> Deserialize<'de> + PartialEq>(value: &T, check: &str) {
    let rendered = dbg!(crate::to_string(value));
    assert_eq!(rendered, check);
    let restored: T = crate::from_str(&rendered).expect("deserialization failed");
    assert_eq!(&restored, value);
}

#[track_caller]
fn roundtrip_pretty<T: Debug + Serialize + for<'de> Deserialize<'de> + PartialEq>(
    value: &T,
    check: &str,
) {
    let rendered = crate::to_string_pretty(value);
    println!("{rendered}");
    assert_eq!(rendered, check);
    let restored: T = crate::from_str(&rendered).expect("deserialization failed");
    assert_eq!(&restored, value);
}

#[track_caller]
fn roundtrip_implicit_map<T: Debug + Serialize + for<'de> Deserialize<'de> + PartialEq>(
    value: &T,
    check: &str,
) {
    let rendered = crate::ser::Config::pretty()
        .implicit_map_at_root(true)
        .serialize(value);
    println!("{rendered}");
    assert_eq!(rendered, check);
    let restored: T = crate::parser::Config::default()
        .allow_implicit_map(true)
        .deserialize(&rendered)
        .expect("deserialization failed");
    assert_eq!(&restored, value);
}

#[track_caller]
fn roundtrip_anonymous_structs<T: Debug + Serialize + for<'de> Deserialize<'de> + PartialEq>(
    value: &T,
    check: &str,
) {
    let rendered = crate::ser::Config::new()
        .anonymous_structs(true)
        .serialize(value);
    println!("{rendered}");
    assert_eq!(rendered, check);
    let restored: T = crate::parser::Config::default()
        .allow_implicit_map(true)
        .deserialize(&rendered)
        .expect("deserialization failed");
    assert_eq!(&restored, value);
}

#[test]
fn struct_of_everything() {
    roundtrip(&StructOfEverything::default(), "StructOfEverything{str:\"\",bytes:b\"\",char:'\\0',u8:0,u16:0,u32:0,u64:0,u128:0,usize:0,i8:0,i16:0,i32:0,i64:0,i128:0,isize:0,bool:false}");
    roundtrip(&StructOfEverything::min(), "StructOfEverything{str:\"\\0\",bytes:b\"\\0\",char:'\\0',u8:0,u16:0,u32:0,u64:0,u128:0,usize:0,i8:-128,i16:-32768,i32:-2147483648,i64:-9223372036854775808,i128:-9223372036854775808,isize:-9223372036854775808,bool:false}");
    roundtrip(&StructOfEverything::max(), "StructOfEverything{str:\"hello 🦀\",bytes:b\"hello, world\",char:'🦀',u8:255,u16:65535,u32:4294967295,u64:18446744073709551615,u128:18446744073709551615,usize:18446744073709551615,i8:127,i16:32767,i32:2147483647,i64:9223372036854775807,i128:9223372036854775807,isize:9223372036854775807,bool:true}");
}

#[test]
fn struct_of_everything_pretty() {
    roundtrip_pretty(&StructOfEverything::default(), "StructOfEverything {\n  str: \"\",\n  bytes: b\"\",\n  char: '\\0',\n  u8: 0,\n  u16: 0,\n  u32: 0,\n  u64: 0,\n  u128: 0,\n  usize: 0,\n  i8: 0,\n  i16: 0,\n  i32: 0,\n  i64: 0,\n  i128: 0,\n  isize: 0,\n  bool: false\n}");
    roundtrip_pretty(&StructOfEverything::min(), "StructOfEverything {\n  str: \"\\0\",\n  bytes: b\"\\0\",\n  char: '\\0',\n  u8: 0,\n  u16: 0,\n  u32: 0,\n  u64: 0,\n  u128: 0,\n  usize: 0,\n  i8: -128,\n  i16: -32768,\n  i32: -2147483648,\n  i64: -9223372036854775808,\n  i128: -9223372036854775808,\n  isize: -9223372036854775808,\n  bool: false\n}");
    roundtrip_pretty(&StructOfEverything::max(), "StructOfEverything {\n  str: \"hello 🦀\",\n  bytes: b\"hello, world\",\n  char: '🦀',\n  u8: 255,\n  u16: 65535,\n  u32: 4294967295,\n  u64: 18446744073709551615,\n  u128: 18446744073709551615,\n  usize: 18446744073709551615,\n  i8: 127,\n  i16: 32767,\n  i32: 2147483647,\n  i64: 9223372036854775807,\n  i128: 9223372036854775807,\n  isize: 9223372036854775807,\n  bool: true\n}");
}

#[test]
fn struct_of_everything_implicit() {
    roundtrip_implicit_map(&StructOfEverything::default(), "str: \"\"\nbytes: b\"\"\nchar: '\\0'\nu8: 0\nu16: 0\nu32: 0\nu64: 0\nu128: 0\nusize: 0\ni8: 0\ni16: 0\ni32: 0\ni64: 0\ni128: 0\nisize: 0\nbool: false\n");
    roundtrip_implicit_map(&StructOfEverything::min(), "str: \"\\0\"\nbytes: b\"\\0\"\nchar: '\\0'\nu8: 0\nu16: 0\nu32: 0\nu64: 0\nu128: 0\nusize: 0\ni8: -128\ni16: -32768\ni32: -2147483648\ni64: -9223372036854775808\ni128: -9223372036854775808\nisize: -9223372036854775808\nbool: false\n");
    roundtrip_implicit_map(&StructOfEverything::max(), "str: \"hello 🦀\"\nbytes: b\"hello, world\"\nchar: '🦀'\nu8: 255\nu16: 65535\nu32: 4294967295\nu64: 18446744073709551615\nu128: 18446744073709551615\nusize: 18446744073709551615\ni8: 127\ni16: 32767\ni32: 2147483647\ni64: 9223372036854775807\ni128: 9223372036854775807\nisize: 9223372036854775807\nbool: true\n");
}

#[test]
fn struct_of_everything_anonymous() {
    roundtrip_anonymous_structs(&StructOfEverything::default(), "{str:\"\",bytes:b\"\",char:'\\0',u8:0,u16:0,u32:0,u64:0,u128:0,usize:0,i8:0,i16:0,i32:0,i64:0,i128:0,isize:0,bool:false}");
    roundtrip_anonymous_structs(&StructOfEverything::min(), "{str:\"\\0\",bytes:b\"\\0\",char:'\\0',u8:0,u16:0,u32:0,u64:0,u128:0,usize:0,i8:-128,i16:-32768,i32:-2147483648,i64:-9223372036854775808,i128:-9223372036854775808,isize:-9223372036854775808,bool:false}");
    roundtrip_anonymous_structs(&StructOfEverything::max(), "{str:\"hello 🦀\",bytes:b\"hello, world\",char:'🦀',u8:255,u16:65535,u32:4294967295,u64:18446744073709551615,u128:18446744073709551615,usize:18446744073709551615,i8:127,i16:32767,i32:2147483647,i64:9223372036854775807,i128:9223372036854775807,isize:9223372036854775807,bool:true}");
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
enum UntaggedEnum {
    Simple(SimpleStruct),
    NewtypeBool(NewtypeBool),
    Unit(UnitStruct),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
enum TaggedEnum {
    Tuple(bool, bool),
    Struct { a: u64 },
    NewtypeStruct(SimpleStruct),
    NewtypeTuple(SimpleTuple),
    NewtypeBool(bool),
    Unit,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct SimpleStruct {
    a: u64,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct SimpleTuple(u64, bool);

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct NewtypeBool(bool);

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct UnitStruct;

#[test]
fn deserialize_any() {
    let untagged: UntaggedEnum = crate::from_str("()").unwrap();
    assert_eq!(untagged, UntaggedEnum::Unit(UnitStruct));
    let untagged: UntaggedEnum = crate::from_str("true").unwrap();
    assert_eq!(untagged, UntaggedEnum::NewtypeBool(NewtypeBool(true)));

    let untagged: UntaggedEnum = crate::from_str("{a:0}").unwrap();
    assert_eq!(untagged, UntaggedEnum::Simple(SimpleStruct { a: 0 }));
    // Serde doesn't support tagged in an untagged context, which makes sense
    // given what it's named. We can't pass the C to the visitor without causing
    // an error within deserialize_any() or causing it to think we're
    // deserializing only a string.
    let untagged: UntaggedEnum = crate::from_str("C{a:0}").unwrap();
    assert_eq!(untagged, UntaggedEnum::Simple(SimpleStruct { a: 0 }));

    // Some and None are special cases
    let untagged: Option<UntaggedEnum> = crate::from_str("None").unwrap();
    assert_eq!(untagged, None);
    let untagged: Option<UntaggedEnum> = crate::from_str("Some(())").unwrap();
    assert_eq!(untagged, Some(UntaggedEnum::Unit(UnitStruct)));
}

#[test]
fn deserialize_tagged() {
    let tagged: TaggedEnum = crate::from_str("Tuple (true, false)").unwrap();
    assert_eq!(tagged, TaggedEnum::Tuple(true, false));
    let tagged: TaggedEnum = crate::from_str("Struct {a: 1}").unwrap();
    assert_eq!(tagged, TaggedEnum::Struct { a: 1 });

    let tagged: TaggedEnum = crate::from_str("NewtypeStruct {a: 1}").unwrap();
    assert_eq!(tagged, TaggedEnum::NewtypeStruct(SimpleStruct { a: 1 }));
    let tagged: TaggedEnum = crate::from_str("NewtypeStruct({a: 1})").unwrap();
    assert_eq!(tagged, TaggedEnum::NewtypeStruct(SimpleStruct { a: 1 }));
    let tagged: TaggedEnum = crate::from_str("NewtypeTuple(1, false)").unwrap();
    assert_eq!(tagged, TaggedEnum::NewtypeTuple(SimpleTuple(1, false)));
    let tagged: TaggedEnum = crate::from_str("NewtypeTuple((1, false))").unwrap();
    assert_eq!(tagged, TaggedEnum::NewtypeTuple(SimpleTuple(1, false)));
    let tagged: TaggedEnum = crate::from_str("NewtypeBool(true)").unwrap();
    assert_eq!(tagged, TaggedEnum::NewtypeBool(true));
    let tagged: TaggedEnum = crate::from_str("Unit").unwrap();
    assert_eq!(tagged, TaggedEnum::Unit);
}

#[test]
fn value_from_serialize() {
    let original = StructOfEverything::default();
    let value = dbg!(Value::from_serialize(&original));
    let from_value: StructOfEverything = value.to_deserialize().unwrap();
    assert_eq!(original, from_value);
}

#[test]
fn implicit_btree_map() {
    roundtrip_implicit_map(
        &BTreeMap::from([(String::from("Hello"), 1), (String::from("World"), 2)]),
        "\"Hello\": 1\n\"World\": 2\n",
    );
}
