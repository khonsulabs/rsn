use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;

#[derive(Debug, Default)]
pub struct Writer<'config> {
    output: String,
    nested: Vec<NestedState>,
    config: Cow<'config, Config>,
}

impl<'config> Writer<'config> {
    pub fn new(config: &'config Config) -> Self {
        Self {
            output: String::new(),
            nested: Vec::new(),
            config: Cow::Borrowed(config),
        }
    }

    pub fn finish(self) -> String {
        assert!(self.nested.is_empty());
        self.output
    }

    pub fn begin_named_map(&mut self, name: &str) {
        self.prepare_to_write_value();
        self.output.push_str(name);
        if matches!(self.config.as_ref(), Config::Pretty { .. }) {
            self.output.push(' ');
        }
        self.output.push('{');
        self.nested.push(NestedState::Map(MapState::Empty));
    }

    pub fn begin_named_tuple(&mut self, name: &str) {
        self.prepare_to_write_value();
        self.output.push_str(name);
        self.output.push('(');
        self.nested.push(NestedState::Tuple(SequenceState::Empty));
    }

    pub fn begin_map(&mut self) {
        self.prepare_to_write_value();
        self.output.push('{');
        self.nested.push(NestedState::Map(MapState::Empty));
    }

    pub fn begin_tuple(&mut self) {
        self.prepare_to_write_value();
        self.output.push('(');
        self.nested.push(NestedState::Tuple(SequenceState::Empty));
    }

    pub fn begin_list(&mut self) {
        self.prepare_to_write_value();
        self.output.push('[');
        self.nested.push(NestedState::List(SequenceState::Empty));
    }

    pub fn write_primitive<P>(&mut self, p: &P)
    where
        P: Primitive + ?Sized,
    {
        self.prepare_to_write_value();
        p.render_to(&mut self.output);
    }

    pub fn write_raw_value(&mut self, ident: &str) {
        self.prepare_to_write_value();
        self.output.push_str(ident);
    }

    fn prepare_to_write_value(&mut self) {
        match self.nested.last_mut() {
            Some(
                NestedState::List(state @ SequenceState::Empty)
                | NestedState::Tuple(state @ SequenceState::Empty),
            ) => {
                *state = SequenceState::NotEmpty;
                self.insert_newline();
            }
            Some(NestedState::List(_) | NestedState::Tuple(_)) => {
                self.output.push(',');
                self.insert_newline();
            }
            Some(NestedState::Map(state @ MapState::Empty)) => {
                *state = MapState::AfterKey;
                self.insert_newline();
            }
            Some(NestedState::Map(state @ MapState::AfterEntry)) => {
                *state = MapState::AfterKey;
                self.output.push(',');
                self.insert_newline();
            }
            Some(NestedState::Map(state @ MapState::AfterKey)) => {
                *state = MapState::AfterEntry;
                if matches!(self.config.as_ref(), Config::Compact) {
                    self.output.push(':');
                } else {
                    self.output.push_str(": ");
                }
            }
            None => {}
        }
    }

    pub fn insert_newline(&mut self) {
        if let Config::Pretty {
            indentation,
            newline,
            ..
        } = self.config.as_ref()
        {
            self.output.push_str(newline);
            for _ in 0..self.nested.len() {
                self.output.push_str(indentation);
            }
        }
    }

    pub fn finish_nested(&mut self) {
        match self.nested.pop().expect("not in a nested state") {
            NestedState::Tuple(state) => {
                if matches!(state, SequenceState::NotEmpty) {
                    self.insert_newline();
                }
                self.output.push(')')
            }
            NestedState::List(state) => {
                if matches!(state, SequenceState::NotEmpty) {
                    self.insert_newline();
                }
                self.output.push(']')
            }
            NestedState::Map(state @ (MapState::AfterEntry | MapState::Empty)) => {
                if matches!(state, MapState::AfterEntry) {
                    self.insert_newline();
                }
                self.output.push('}')
            }
            NestedState::Map(_) => unreachable!("map entry not complete"),
        }
    }
}

pub trait Primitive {
    fn render_to(&self, buffer: &mut String);
}

macro_rules! impl_primitive_using_to_string {
    ($type:ty) => {
        impl Primitive for $type {
            fn render_to(&self, buffer: &mut String) {
                buffer.push_str(&self.to_string());
            }
        }
    };
}

impl_primitive_using_to_string!(u8);
impl_primitive_using_to_string!(u16);
impl_primitive_using_to_string!(u32);
impl_primitive_using_to_string!(u64);
impl_primitive_using_to_string!(u128);
impl_primitive_using_to_string!(usize);
impl_primitive_using_to_string!(i8);
impl_primitive_using_to_string!(i16);
impl_primitive_using_to_string!(i32);
impl_primitive_using_to_string!(i64);
impl_primitive_using_to_string!(i128);
impl_primitive_using_to_string!(isize);
impl_primitive_using_to_string!(f64);
impl_primitive_using_to_string!(f32);

impl Primitive for str {
    fn render_to(&self, buffer: &mut String) {
        buffer.reserve(self.len() + 2);
        buffer.push('"');
        for ch in self.chars() {
            escape_string_char(ch, buffer);
        }
        buffer.push('"');
    }
}

impl Primitive for bool {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(if *self { "true" } else { "false" });
    }
}

impl Primitive for [u8] {
    fn render_to(&self, buffer: &mut String) {
        buffer.reserve(self.len() + 3);
        buffer.push_str("b\"");
        for byte in self {
            match DEFAULT_STRING_ESCAPE_HANDLING.get(usize::from(*byte)) {
                Some(Some(escaped)) => {
                    buffer.push_str(escaped);
                }
                Some(None) => {
                    buffer.push(char::from(*byte));
                }
                None => {
                    // Non-ASCII, must be hex-escaped.
                    write!(buffer, "\\x{byte:02x}").expect("failed to format");
                }
            }
        }
        buffer.push('"');
    }
}

#[inline]
fn escape_string_char(ch: char, buffer: &mut String) {
    if let Ok(cp) = usize::try_from(u32::from(ch)) {
        if let Some(Some(escaped)) = DEFAULT_STRING_ESCAPE_HANDLING.get(cp) {
            buffer.push_str(escaped);
            return;
        }
    }

    let mut utf8_bytes = [0; 8];
    buffer.push_str(ch.encode_utf8(&mut utf8_bytes));
}

impl Primitive for char {
    fn render_to(&self, buffer: &mut String) {
        buffer.push('\'');
        escape_string_char(*self, buffer);
        buffer.push('\'');
    }
}

#[rustfmt::skip]
static DEFAULT_STRING_ESCAPE_HANDLING: [Option<&'static str>; 128] = [
    // 0x0         1              2              3              4              5              6              7
    Some("\\0"),   Some("\\x01"), Some("\\x02"), Some("\\x03"), Some("\\x04"), Some("\\x05"), Some("\\x06"), Some("\\x07"),
    // 0x8         9              A              B              C              D              E              F
    Some("\\x08"), Some("\\t"),   Some("\\n"),   Some("\\x0b"), Some("\\x0c"), Some("\\r"),   Some("\\x0e"), Some("\\x0f"),
    // 0x10
    Some("\\x10"), Some("\\x11"), Some("\\x12"), Some("\\x13"), Some("\\x14"), Some("\\x15"), Some("\\x16"), Some("\\x17"),
    Some("\\x18"), Some("\\x19"), Some("\\x1a"), Some("\\x1b"), Some("\\x1c"), Some("\\x1d"), Some("\\x1e"), Some("\\x1f"),
    // 0x20
    None,          None,          Some("\\\""),  None,          None,          None,          None,          None,
    None,          None,          None,          None,          None,          None,          None,          None,
    // 0x30
    None,          None,          None,          None,          None,          None,          None,          None,
    None,          None,          None,          None,          None,          None,          None,          None,
    // 0x40
    None,          None,          None,          None,          None,          None,          None,          None,
    None,          None,          None,          None,          None,          None,          None,          None,
    // 0x50
    None,          None,          None,          None,          None,          None,          None,          None,
    None,          None,          None,          None,          Some("\\\\"),  None,          None,          None,
    // 0x60
    None,          None,          None,          None,          None,          None,          None,          None,
    None,          None,          None,          None,          None,          None,          None,          None,
    // 0x70
    None,          None,          None,          None,          None,          None,          None,          None,
    None,          None,          None,          None,          None,          None,          None,          Some("\\x7f"),
];

#[derive(Debug)]
enum NestedState {
    Tuple(SequenceState),
    List(SequenceState),
    Map(MapState),
}

#[derive(Debug)]
enum SequenceState {
    Empty,
    NotEmpty,
}

#[derive(Debug)]
enum MapState {
    Empty,
    AfterEntry,
    AfterKey,
}

#[derive(Debug, Default, Clone)]
pub enum Config {
    #[default]
    Compact,
    Pretty {
        indentation: Cow<'static, str>,
        newline: Cow<'static, str>,
    },
}

#[test]
fn string_rendering() {
    use crate::tokenizer::{Token, TokenKind, Tokenizer};
    let mut to_encode = String::new();
    for ch in 0_u8..128 {
        to_encode.push(ch as char);
    }
    to_encode.push('\u{1_F980}');
    let mut rendered = String::new();
    to_encode.render_to(&mut rendered);
    assert_eq!(
        rendered,
        "\"\\0\\x01\\x02\\x03\\x04\\x05\\x06\\x07\\x08\\t\\n\\x0b\\x0c\\r\\x0e\\x0f\\x10\\x11\\x12\\x13\\x14\\x15\\x16\\x17\\x18\\x19\\x1a\\x1b\\x1c\\x1d\\x1e\\x1f !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\x7f🦀\""
    );
    let Some(Ok(Token { kind: TokenKind::String(parsed), .. })) = Tokenizer::full(&rendered).next() else { unreachable!("failed to parse rendered string") };
    assert_eq!(parsed, to_encode);
}

#[test]
fn byte_rendering() {
    use crate::tokenizer::{Token, TokenKind, Tokenizer};
    let mut to_encode = Vec::new();
    for ch in 0_u8..255 {
        to_encode.push(ch);
    }
    let mut rendered = String::new();
    to_encode.render_to(&mut rendered);
    assert_eq!(
        rendered,
        "b\"\\0\\x01\\x02\\x03\\x04\\x05\\x06\\x07\\x08\\t\\n\\x0b\\x0c\\r\\x0e\\x0f\\x10\\x11\\x12\\x13\\x14\\x15\\x16\\x17\\x18\\x19\\x1a\\x1b\\x1c\\x1d\\x1e\\x1f !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\x7f\\x80\\x81\\x82\\x83\\x84\\x85\\x86\\x87\\x88\\x89\\x8a\\x8b\\x8c\\x8d\\x8e\\x8f\\x90\\x91\\x92\\x93\\x94\\x95\\x96\\x97\\x98\\x99\\x9a\\x9b\\x9c\\x9d\\x9e\\x9f\\xa0\\xa1\\xa2\\xa3\\xa4\\xa5\\xa6\\xa7\\xa8\\xa9\\xaa\\xab\\xac\\xad\\xae\\xaf\\xb0\\xb1\\xb2\\xb3\\xb4\\xb5\\xb6\\xb7\\xb8\\xb9\\xba\\xbb\\xbc\\xbd\\xbe\\xbf\\xc0\\xc1\\xc2\\xc3\\xc4\\xc5\\xc6\\xc7\\xc8\\xc9\\xca\\xcb\\xcc\\xcd\\xce\\xcf\\xd0\\xd1\\xd2\\xd3\\xd4\\xd5\\xd6\\xd7\\xd8\\xd9\\xda\\xdb\\xdc\\xdd\\xde\\xdf\\xe0\\xe1\\xe2\\xe3\\xe4\\xe5\\xe6\\xe7\\xe8\\xe9\\xea\\xeb\\xec\\xed\\xee\\xef\\xf0\\xf1\\xf2\\xf3\\xf4\\xf5\\xf6\\xf7\\xf8\\xf9\\xfa\\xfb\\xfc\\xfd\\xfe\""
    );
    let Some(Ok(Token { kind: TokenKind::Bytes(parsed), .. })) = Tokenizer::full(&rendered).next() else { unreachable!("failed to parse rendered bytes") };
    assert_eq!(parsed, to_encode);
}
