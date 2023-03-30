use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::ops::Range;

use crate::tokenizer::Integer;
pub type Value<'a> = Annotated<'a, Literal<'a>>;
pub type OwnedValue = Value<'static>;

pub struct Annotated<'a, T> {
    pub attributes: Vec<Attribute<'a>>,
    pub location: Range<usize>,
    pub literal: T,
}
pub enum Literal<'a> {
    Integer(Integer),
    Float(f64),
    Bool(bool),
    Char(char),
    Byte(u8),
    String(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Type(Type<'a>),
    Tuple(List<'a>),
    Array(List<'a>),
}

pub struct Type<'a> {
    pub name: Identifier<'a>,
    pub contents: StructContents<'a>,
}

pub enum StructContents<'a> {
    Map(Map<'a>),
    Tuple(List<'a>),
}

pub struct Identifier<'a>(pub Cow<'a, str>);

pub struct Map<'a>(pub Vec<(Value<'a>, Value<'a>)>);

pub struct List<'a>(pub Vec<Value<'a>>);

pub struct Attribute<'a> {
    pub name: Cow<'a, str>,
    pub contents: Cow<'a, str>,
}
