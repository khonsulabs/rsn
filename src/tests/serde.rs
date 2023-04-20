use core::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
enum UntaggedEnum {
    Simple(SimpleStruct),
    NewtypeBool(NewtypeBool),
    Unit(UnitStruct),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct SimpleStruct {
    a: u64,
}

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
