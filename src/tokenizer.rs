use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt::Display;
use core::ops::Range;

use unicode_ident::{is_xid_continue, is_xid_start};

use crate::tokenizer::char_iterator::CharIterator;

mod char_iterator;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Token<'a> {
    pub location: Range<usize>,
    pub kind: TokenKind<'a>,
}

impl<'a> Token<'a> {
    pub const fn new(location: Range<usize>, kind: TokenKind<'a>) -> Self {
        Self { location, kind }
    }
}

#[derive(Clone, Debug)]
pub enum TokenKind<'a> {
    Integer(Integer),
    Float(f64),
    Bool(bool),
    Character(char),
    Colon,
    Comma,
    Byte(u8),
    String(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Identifier(Cow<'a, str>),
    Open(Balanced),
    Close(Balanced),
    Comment(Cow<'a, str>), // TODO needs comment kind -- block vs line
    Whitespace,
}

impl<'a> Eq for TokenKind<'a> {}

impl<'a> PartialEq for TokenKind<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0.total_cmp(r0).is_eq(),
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Character(l0), Self::Character(r0)) => l0 == r0,
            (Self::Byte(l0), Self::Byte(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Bytes(l0), Self::Bytes(r0)) => l0 == r0,
            (Self::Identifier(l0), Self::Identifier(r0)) => l0 == r0,
            (Self::Open(l0), Self::Open(r0)) => l0 == r0,
            (Self::Close(l0), Self::Close(r0)) => l0 == r0,
            (Self::Comment(l0), Self::Comment(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Integer {
    Usize(usize),
    Isize(isize),
    UnsignedLarge(UnsignedLarge),
    SignedLarge(SignedLarge),
}

macro_rules! fn_integer_into {
    ($name:ident, $type:ty) => {
        #[inline]
        pub fn $name(self) -> Option<$type> {
            match self {
                Integer::Usize(value) => value.try_into().ok(),
                Integer::Isize(value) => value.try_into().ok(),
                Integer::UnsignedLarge(value) => value.try_into().ok(),
                Integer::SignedLarge(value) => value.try_into().ok(),
            }
        }
    };
}

impl Integer {
    fn_integer_into!(into_u8, u8);

    fn_integer_into!(into_u16, u16);

    fn_integer_into!(into_u32, u32);

    fn_integer_into!(into_u64, u64);

    fn_integer_into!(into_u128, u128);

    fn_integer_into!(into_usize, usize);

    fn_integer_into!(into_i8, i8);

    fn_integer_into!(into_i16, i16);

    fn_integer_into!(into_i32, i32);

    fn_integer_into!(into_i64, i64);

    fn_integer_into!(into_i128, i128);

    fn_integer_into!(into_isize, isize);

    #[inline]
    pub const fn is_zero(self) -> bool {
        match self {
            Integer::Usize(value) => value == 0,
            Integer::Isize(value) => value == 0,
            Integer::UnsignedLarge(value) => value == 0,
            Integer::SignedLarge(value) => value == 0,
        }
    }
}

#[cfg(feature = "integer128")]
type SignedLarge = i128;
#[cfg(feature = "integer128")]
type UnsignedLarge = u128;

#[cfg(not(feature = "integer128"))]
type SignedLarge = i64;
#[cfg(not(feature = "integer128"))]
type UnsignedLarge = u64;

impl From<usize> for Integer {
    fn from(value: usize) -> Self {
        Self::Usize(value)
    }
}

impl From<isize> for Integer {
    fn from(value: isize) -> Self {
        Self::Isize(value)
    }
}

impl From<SignedLarge> for Integer {
    fn from(value: SignedLarge) -> Self {
        Self::SignedLarge(value)
    }
}

impl From<UnsignedLarge> for Integer {
    fn from(value: UnsignedLarge) -> Self {
        Self::UnsignedLarge(value)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Balanced {
    Paren,
    Brace,
    Bracket,
}

#[derive(Clone)]
pub struct Tokenizer<'a, const INCLUDE_ALL: bool> {
    chars: CharIterator<'a>,
    scratch: String,
}

impl<'a> Tokenizer<'a, false> {
    pub fn minified(source: &'a str) -> Self {
        Self::new(source)
    }
}

impl<'a> Tokenizer<'a, true> {
    pub fn full(source: &'a str) -> Self {
        Self::new(source)
    }
}

impl<'a, const INCLUDE_ALL: bool> Tokenizer<'a, INCLUDE_ALL> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: CharIterator::new(source),
            scratch: String::new(),
        }
    }

    pub const fn current_offset(&self) -> usize {
        self.chars.current_offset()
    }

    fn next_or_eof(&mut self) -> Result<char, Error> {
        self.chars
            .next()
            .ok_or_else(|| Error::new(self.chars.last_char_range(), ErrorKind::UnexpectedEof))
    }

    fn tokenize_positive_integer<I>(&mut self, mut value: I) -> Result<Token<'a>, Error>
    where
        I: Integral,
        Integer: From<I> + From<I::Larger>,
    {
        let mut has_decimal = false;
        let mut has_exponent = false;
        let mut had_underscores = false;
        let mut overflowing = false;
        while let Some(ch) = self.chars.peek() {
            let digit_value = (ch as u8).wrapping_sub(b'0');
            if digit_value < 10 {
                if let Some(new_value) = value
                    .checked_mul(I::from(10))
                    .and_then(|value| value.checked_add(I::from(digit_value)))
                {
                    value = new_value;
                    self.chars.next();
                } else {
                    overflowing = true;
                    break;
                }
            } else if ch == '.' {
                has_decimal = true;
                self.chars.next();
                break;
            } else if ch == 'e' || ch == 'E' {
                has_decimal = true;
                has_exponent = true;
                self.chars.next();
                break;
            } else if ch == '_' {
                self.chars.next();
                had_underscores = true;
            } else {
                break;
            }
        }

        if overflowing {
            let mut value: I::Larger = value.into_larger();
            while let Some(ch) = self.chars.peek() {
                let digit_value = (ch as u8).wrapping_sub(b'0');
                if digit_value < 10 {
                    if let Some(new_value) = value
                        .checked_mul(<I::Larger>::from(10))
                        .and_then(|value| value.checked_add(<I::Larger>::from(digit_value)))
                    {
                        value = new_value;
                        self.chars.next();
                    } else {
                        todo!("overflowed large")
                    }
                } else if ch == '.' {
                    has_decimal = true;
                    self.chars.next();
                    break;
                } else if ch == 'e' || ch == 'E' {
                    has_decimal = true;
                    has_exponent = true;
                    self.chars.next();
                    break;
                } else if ch == '_' {
                    self.chars.next();
                    had_underscores = true;
                } else {
                    break;
                }
            }

            if !has_decimal {
                return Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Integer(Integer::from(value)),
                ));
            }
        }

        if has_decimal {
            self.tokenize_float(had_underscores, has_exponent)
        } else {
            Ok(Token::new(
                self.chars.marked_range(),
                TokenKind::Integer(Integer::from(value)),
            ))
        }
    }

    fn tokenize_negative_integer<I>(&mut self, mut value: I) -> Result<Token<'a>, Error>
    where
        I: Integral,
        Integer: From<I> + From<I::Larger>,
    {
        let mut has_decimal = false;
        let mut has_exponent = false;
        let mut overflowing = false;
        let mut had_underscores = false;
        while let Some(ch) = self.chars.peek() {
            let digit_value = (ch as u8).wrapping_sub(b'0');
            if digit_value < 10 {
                if let Some(new_value) = value
                    .checked_mul(I::from(10))
                    .and_then(|value| value.checked_sub(I::from(digit_value)))
                {
                    value = new_value;
                    self.chars.next();
                } else {
                    overflowing = true;
                    break;
                }
            } else if ch == '.' {
                has_decimal = true;
                self.chars.next();
                break;
            } else if ch == 'e' || ch == 'E' {
                has_decimal = true;
                has_exponent = true;
                self.chars.next();
                break;
            } else if ch == '_' {
                had_underscores = true;
                self.chars.next();
            } else {
                break;
            }
        }

        if overflowing {
            let mut value: I::Larger = value.into_larger();
            while let Some(ch) = self.chars.peek() {
                let digit_value = (ch as u8).wrapping_sub(b'0');
                if digit_value < 10 {
                    if let Some(new_value) = value
                        .checked_mul(<I::Larger>::from(10))
                        .and_then(|value| value.checked_sub(<I::Larger>::from(digit_value)))
                    {
                        value = new_value;
                        self.chars.next();
                    } else {
                        todo!("overflowed large")
                    }
                } else if ch == '.' {
                    has_decimal = true;
                    self.chars.next();
                    break;
                } else if ch == 'e' || ch == 'E' {
                    has_decimal = true;
                    has_exponent = true;
                    self.chars.next();
                    break;
                } else if ch == '_' {
                    had_underscores = true;
                    self.chars.next();
                } else {
                    break;
                }
            }

            if !has_decimal {
                return Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Integer(Integer::from(value)),
                ));
            }
        }

        if has_decimal {
            self.tokenize_float(had_underscores, has_exponent)
        } else {
            Ok(Token::new(
                self.chars.marked_range(),
                TokenKind::Integer(Integer::from(value)),
            ))
        }
    }

    fn tokenize_float(
        &mut self,
        had_underscores: bool,
        mut has_exponent: bool,
    ) -> Result<Token<'a>, Error> {
        self.scratch.clear();
        let already_read_chars = self.chars.marked_str();
        if had_underscores {
            self.scratch.extend(
                already_read_chars
                    .chars()
                    .filter_map(|ch| (ch != '_').then_some(ch)),
            );
        } else {
            self.scratch.push_str(already_read_chars);
        }

        if !has_exponent {
            // Read any decimal digits
            while let Some(ch) = self.chars.peek() {
                if ch.is_ascii_digit() {
                    self.scratch.push(ch);
                    self.chars.next();
                } else if !has_exponent && ch == 'e' || ch == 'E' {
                    self.scratch.push(ch);
                    has_exponent = true;
                    self.chars.next();

                    break;
                } else {
                    break;
                }
            }
        }

        if has_exponent {
            // Handle the exponent sign
            if let Some(ch) = self.chars.peek() {
                if ch == '+' || ch == '-' {
                    self.scratch.push(ch);
                    self.chars.next();
                }
            }

            // Require at least one digit for the exponent, but allow
            // skipping underscores.
            let mut has_exponent_digit = false;
            while let Some(ch) = self.chars.peek() {
                let is_digit = ch.is_ascii_digit();

                if is_digit || ch == '_' {
                    has_exponent_digit |= is_digit;
                    self.scratch.push(ch);
                    self.chars.next();
                } else {
                    break;
                }
            }

            if !has_exponent_digit {
                todo!("expected exponent digit")
            }
        }

        let parsed = self.scratch.parse::<f64>().unwrap(); // TODO handle this error

        Ok(Token::new(
            self.chars.marked_range(),
            TokenKind::Float(parsed),
        ))
    }

    fn tokenize_hex_large_number<const BITS: u32>(
        &mut self,
        signed: bool,
        negative: bool,
        value: usize,
        first_hex_value: u8,
    ) -> Result<Token<'a>, Error> {
        assert!(BITS == 1 || BITS == 3 || BITS == 4);
        let max = 2_u8.pow(BITS);
        let mut value = value as UnsignedLarge;
        value <<= BITS;
        value |= first_hex_value as UnsignedLarge;

        while let Some(ch) = self.chars.peek() {
            let ch = ch as u8;

            let hex_value = if let Some(zero_relative) = ch.checked_sub(b'0') {
                if zero_relative <= 9 {
                    zero_relative
                } else if ch == b'_' {
                    self.chars.next();
                    continue;
                } else {
                    // Setting the 6th bit ensures the ascii letters are always lower
                    // cased. This allows skipping one more comparison.
                    let Some(a_relative) = (ch | 0b10_0000).checked_sub(b'a') else { break };
                    if a_relative <= 5 {
                        a_relative + 10
                    } else {
                        break;
                    }
                }
            } else {
                break;
            };

            self.chars.next();
            if let Some(next_value) = value
                .checked_mul(max as UnsignedLarge)
                .and_then(|value| value.checked_add(hex_value as UnsignedLarge))
            {
                value = next_value
            } else {
                // Overflowed
                todo!("error: overflowed u128")
            }
        }

        Ok(Token::new(
            self.chars.marked_range(),
            TokenKind::Integer(match (signed, negative) {
                (_, true) => Integer::SignedLarge(-(value as SignedLarge)),
                (true, _) => Integer::SignedLarge(value as SignedLarge),
                (false, _) => Integer::UnsignedLarge(value),
            }),
        ))
    }

    fn tokenize_radix_number<const BITS: u32>(
        &mut self,
        signed: bool,
        negative: bool,
    ) -> Result<Token<'a>, Error> {
        assert!(BITS == 1 || BITS == 3 || BITS == 4);
        let max = 2_u8.pow(BITS);
        let mut value = 0usize;
        let mut read_at_least_one_digit = false;

        while let Some(ch) = self.chars.peek() {
            let ch = ch as u8;

            let hex_value = if let Some(zero_relative) = ch.checked_sub(b'0') {
                if zero_relative < 10.min(max) {
                    zero_relative
                } else if ch == b'_' {
                    self.chars.next();
                    continue;
                } else if BITS == 4 {
                    // Setting the 6th bit ensures the ascii letters are always lower
                    // cased. This allows skipping one more comparison.
                    let Some(a_relative) = (ch | 0b10_0000).checked_sub(b'a') else { break };
                    if a_relative <= 5 {
                        a_relative + 10
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            };

            read_at_least_one_digit = true;
            self.chars.next();
            if let Some(next_value) = value
                .checked_mul(max as usize)
                .and_then(|value| value.checked_add(hex_value as usize))
            {
                value = next_value
            } else {
                // Overflowed
                return self.tokenize_hex_large_number::<BITS>(signed, negative, value, hex_value);
            }
        }

        if read_at_least_one_digit {
            Ok(Token::new(
                self.chars.marked_range(),
                TokenKind::Integer(match (signed, negative) {
                    (_, true) => Integer::Isize(-(value as isize)),
                    (true, _) => Integer::Isize(value as isize),
                    (false, _) => Integer::Usize(value),
                }),
            ))
        } else {
            todo!("expected hex digit")
        }
    }

    fn tokenize_leading_zero_number(
        &mut self,
        signed: bool,
        negative: bool,
    ) -> Result<Token<'a>, Error> {
        match self.chars.peek() {
            Some('x') | Some('X') => {
                self.chars.next();
                return self.tokenize_radix_number::<4>(signed, negative);
            }
            Some('b') | Some('B') => {
                self.chars.next();
                return self.tokenize_radix_number::<1>(signed, negative);
            }
            Some('o') | Some('O') => {
                self.chars.next();
                return self.tokenize_radix_number::<3>(signed, negative);
            }
            _ => {}
        }

        match (signed, negative) {
            (_, true) => self.tokenize_negative_integer(0isize),
            (true, _) => self.tokenize_positive_integer(0isize),
            (false, _) => self.tokenize_positive_integer(0usize),
        }
    }

    fn tokenize_number(&mut self, start_char: char) -> Result<Token<'a>, Error> {
        let negative = start_char == '-';
        let signed = negative || start_char == '+';

        if signed {
            let next_char = self.next_or_eof()?;
            if next_char == '0' {
                self.tokenize_leading_zero_number(signed, negative)
            } else if next_char.is_ascii_digit() {
                let value = (next_char as u8 - b'0') as isize;
                if negative {
                    self.tokenize_negative_integer(-value)
                } else {
                    self.tokenize_positive_integer(value)
                }
            } else {
                Err(Error::new(
                    self.chars.marked_range(),
                    ErrorKind::ExpectedDigitAfterSign,
                ))
            }
        } else if start_char == '0' {
            self.tokenize_leading_zero_number(signed, negative)
        } else {
            let value = (start_char as u8 - b'0') as usize;
            self.tokenize_positive_integer(value)
        }
    }

    fn tokenize_char(&mut self) -> Result<Token<'a>, Error> {
        todo!()
    }

    fn tokenize_byte(&mut self) -> Result<Token<'a>, Error> {
        todo!()
    }

    fn tokenize_string(&mut self) -> Result<Token<'a>, Error> {
        todo!()
    }

    fn tokenize_byte_string(&mut self) -> Result<Token<'a>, Error> {
        todo!()
    }

    fn tokenize_raw_string(&mut self) -> Result<Token<'a>, Error> {
        todo!()
    }

    fn tokenize_identifier(&mut self, initial_char: Option<char>) -> Result<Token<'a>, Error> {
        let (require_start, initial_char, is_raw, initial_char_index) =
            if let Some(ch) = initial_char {
                (ch != '_', ch, false, self.chars.last_offset())
            } else {
                let initial = self.next_or_eof()?;
                (true, initial, true, self.chars.last_offset())
            };
        // Validate the first character
        let start_is_valid = if require_start {
            is_xid_start(initial_char)
        } else {
            is_xid_continue(initial_char)
        };

        if start_is_valid {
            while let Some(ch) = self.chars.peek() {
                if is_xid_continue(ch) {
                    self.chars.next();
                } else {
                    break;
                }
            }

            let source = &self.chars.source[initial_char_index..self.chars.current_offset()];

            Ok(Token::new(
                self.chars.marked_range(),
                match source {
                    "true" if !is_raw => TokenKind::Bool(true),
                    "false" if !is_raw => TokenKind::Bool(false),
                    _ => TokenKind::Identifier(Cow::Borrowed(source)),
                },
            ))
        } else {
            Err(Error {
                location: initial_char_index..self.chars.current_offset(),
                kind: ErrorKind::Unexpected(initial_char),
            })
        }
    }
}

impl<'a, const INCLUDE_ALL: bool> Iterator for Tokenizer<'a, INCLUDE_ALL> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.chars.mark_start();
            let ch = self.chars.next()?;
            let result = match ch {
                '0'..='9' | '-' | '+' => self.tokenize_number(ch),
                '"' => self.tokenize_string(),
                '\'' => self.tokenize_char(),
                'r' => match self.chars.peek() {
                    Some('"') => {
                        self.chars.next();
                        self.tokenize_raw_string()
                    }
                    Some('#') => {
                        self.chars.next();
                        self.tokenize_identifier(None)
                    }
                    _ => self.tokenize_identifier(Some(ch)),
                },
                'b' => match self.chars.peek() {
                    Some('"') => {
                        self.chars.next();
                        self.tokenize_byte_string()
                    }
                    Some('\'') => {
                        self.chars.next();
                        self.tokenize_byte()
                    }
                    _ => self.tokenize_identifier(Some(ch)),
                },
                '(' => Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Open(Balanced::Paren),
                )),
                ')' => Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Close(Balanced::Paren),
                )),
                '{' => Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Open(Balanced::Brace),
                )),
                '}' => Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Close(Balanced::Brace),
                )),
                '[' => Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Open(Balanced::Bracket),
                )),
                ']' => Ok(Token::new(
                    self.chars.marked_range(),
                    TokenKind::Close(Balanced::Bracket),
                )),
                ':' => Ok(Token::new(self.chars.marked_range(), TokenKind::Colon)),
                ',' => Ok(Token::new(self.chars.marked_range(), TokenKind::Comma)),
                ch if is_rust_whitespace(ch) => {
                    loop {
                        match self.chars.peek() {
                            Some(ch) if is_rust_whitespace(ch) => {
                                self.chars.next();
                            }
                            _ => break,
                        }
                    }
                    if INCLUDE_ALL {
                        Ok(Token::new(self.chars.marked_range(), TokenKind::Whitespace))
                    } else {
                        continue;
                    }
                }
                '/' => todo!("comments"),
                ch => self.tokenize_identifier(Some(ch)),
            };
            break Some(result);
        }
    }
}

fn is_rust_whitespace(ch: char) -> bool {
    // https://doc.rust-lang.org/reference/whitespace.html
    matches!(
        ch,
        '\t' | '\n'
            | '\u{b}'
            | '\u{c}'
            | '\r'
            | ' '
            | '\u{85}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    pub location: Range<usize>,
    pub kind: ErrorKind,
}

impl Error {
    pub const fn new(location: Range<usize>, kind: ErrorKind) -> Self {
        Self { location, kind }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    UnexpectedEof,
    Unexpected(char),
    ExpectedDigitAfterSign,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorKind::UnexpectedEof => f.write_str("unexpected eof"),
            ErrorKind::Unexpected(ch) => write!(f, "unexpected `{ch}`"),
            ErrorKind::ExpectedDigitAfterSign => f.write_str("expected digit after sign"),
        }
    }
}

pub trait Integral: From<u8> + Copy {
    type Larger: Integral;
    fn into_larger(self) -> Self::Larger;
    fn checked_mul(self, other: Self) -> Option<Self>;
    fn checked_add(self, other: Self) -> Option<Self>;
    fn checked_sub(self, other: Self) -> Option<Self>;
}

impl Integral for usize {
    type Larger = UnsignedLarge;

    fn into_larger(self) -> Self::Larger {
        self as UnsignedLarge
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        self.checked_add(other)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_sub(other)
    }
}

impl Integral for UnsignedLarge {
    type Larger = Self;

    fn into_larger(self) -> Self::Larger {
        self
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        self.checked_add(other)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_sub(other)
    }
}

impl Integral for isize {
    type Larger = SignedLarge;

    fn into_larger(self) -> Self::Larger {
        self as SignedLarge
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        self.checked_add(other)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_sub(other)
    }
}

impl Integral for SignedLarge {
    type Larger = Self;

    fn into_larger(self) -> Self::Larger {
        self
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        self.checked_add(other)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_sub(other)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;
    #[track_caller]
    fn test_tokens(source: &str, tokens: &[Token<'_>]) {
        assert_eq!(
            &Tokenizer::minified(source)
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            tokens
        );
    }
    #[track_caller]
    fn test_tokens_err(source: &str, location: Range<usize>, kind: ErrorKind) {
        let err = Tokenizer::minified(source)
            .collect::<Result<Vec<_>, _>>()
            .expect_err("source did not error");
        assert_eq!(err.kind, kind);
        assert_eq!(err.location, location);
    }

    #[test]
    fn identifiers() {
        test_tokens("true", &[Token::new(0..4, TokenKind::Bool(true))]);
        test_tokens("false", &[Token::new(0..5, TokenKind::Bool(false))]);

        test_tokens(
            "r#true",
            &[Token::new(
                0..6,
                TokenKind::Identifier(Cow::Borrowed("true")),
            )],
        );
        test_tokens(
            "r#false",
            &[Token::new(
                0..7,
                TokenKind::Identifier(Cow::Borrowed("false")),
            )],
        );

        test_tokens(
            "_",
            &[Token::new(0..1, TokenKind::Identifier(Cow::Borrowed("_")))],
        );

        test_tokens(
            "_0",
            &[Token::new(0..2, TokenKind::Identifier(Cow::Borrowed("_0")))],
        );

        test_tokens_err("=", 0..1, ErrorKind::Unexpected('='));
    }

    #[test]
    fn integers() {
        test_tokens(
            "0",
            &[Token::new(0..1, TokenKind::Integer(Integer::Usize(0)))],
        );
        test_tokens(
            "9",
            &[Token::new(0..1, TokenKind::Integer(Integer::Usize(9)))],
        );
        test_tokens(
            "10",
            &[Token::new(0..2, TokenKind::Integer(Integer::Usize(10)))],
        );
        test_tokens(
            "99",
            &[Token::new(0..2, TokenKind::Integer(Integer::Usize(99)))],
        );
        test_tokens(
            "+0",
            &[Token::new(0..2, TokenKind::Integer(Integer::Isize(0)))],
        );
        test_tokens(
            "+9",
            &[Token::new(0..2, TokenKind::Integer(Integer::Isize(9)))],
        );
        test_tokens(
            "-0",
            &[Token::new(0..2, TokenKind::Integer(Integer::Isize(0)))],
        );
        test_tokens(
            "-9",
            &[Token::new(0..2, TokenKind::Integer(Integer::Isize(-9)))],
        );
        test_tokens(
            "-10",
            &[Token::new(0..3, TokenKind::Integer(Integer::Isize(-10)))],
        );
        test_tokens(
            "-99",
            &[Token::new(0..3, TokenKind::Integer(Integer::Isize(-99)))],
        );

        // Test 16-bit integer maximums
        test_tokens(
            "+32_767",
            &[Token::new(0..7, TokenKind::Integer(Integer::Isize(32_767)))],
        );
        test_tokens(
            "-32_768",
            &[Token::new(
                0..7,
                TokenKind::Integer(Integer::Isize(-32_768)),
            )],
        );
        test_tokens(
            "65_535",
            &[Token::new(0..6, TokenKind::Integer(Integer::Usize(65_535)))],
        );

        // Test 32-bit integer maximums
        test_tokens(
            "+2_147_483_647",
            &[Token::new(
                0..14,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Isize(2_147_483_647)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::SignedLarge(2_147_483_647)),
            )],
        );
        test_tokens(
            "-2_147_483_648",
            &[Token::new(
                0..14,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-2_147_483_648)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-2_147_483_648)),
            )],
        );
        test_tokens(
            "4_294_967_295",
            &[Token::new(
                0..13,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Usize(4_294_967_295)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::UnsignedLarge(4_294_967_295)),
            )],
        );

        // Test 64-bit integer maximums
        test_tokens(
            "+9_223_372_036_854_775_807",
            &[Token::new(
                0..26,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(9_223_372_036_854_775_807)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(9_223_372_036_854_775_807)),
            )],
        );
        test_tokens(
            "-9_223_372_036_854_775_808",
            &[Token::new(
                0..26,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-9_223_372_036_854_775_808)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-9_223_372_036_854_775_808)),
            )],
        );
        test_tokens(
            "18_446_744_073_709_551_615",
            &[Token::new(
                0..26,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Usize(18_446_744_073_709_551_615)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::UnsignedLarge(18_446_744_073_709_551_615)),
            )],
        );

        #[cfg(feature = "integer128")]
        {
            test_tokens(
                "+9_223_372_036_854_775_808",
                &[Token::new(
                    0..26,
                    TokenKind::Integer(Integer::SignedLarge(9_223_372_036_854_775_808)),
                )],
            );
            test_tokens(
                "-9_223_372_036_854_775_809",
                &[Token::new(
                    0..26,
                    TokenKind::Integer(Integer::SignedLarge(-9_223_372_036_854_775_809)),
                )],
            );
            test_tokens(
                "18_446_744_073_709_551_616",
                &[Token::new(
                    0..26,
                    TokenKind::Integer(Integer::UnsignedLarge(18_446_744_073_709_551_616)),
                )],
            );
        }
    }

    #[test]
    fn hex_integers() {
        test_tokens(
            "0x1",
            &[Token::new(0..3, TokenKind::Integer(Integer::Usize(1)))],
        );
        test_tokens(
            "0X12",
            &[Token::new(0..4, TokenKind::Integer(Integer::Usize(0x12)))],
        );
        test_tokens(
            "0x12_3",
            &[Token::new(0..6, TokenKind::Integer(Integer::Usize(0x123)))],
        );
        test_tokens(
            "0xaBc",
            &[Token::new(0..5, TokenKind::Integer(Integer::Usize(0xabc)))],
        );

        // Test 16-bit integer maximums
        test_tokens(
            "+0xFFFF",
            &[Token::new(0..7, TokenKind::Integer(Integer::Isize(0xFFFF)))],
        );
        test_tokens(
            "-0xFFFF",
            &[Token::new(
                0..7,
                TokenKind::Integer(Integer::Isize(-0xFFFF)),
            )],
        );
        test_tokens(
            "0xFFFF",
            &[Token::new(0..6, TokenKind::Integer(Integer::Usize(0xFFFF)))],
        );

        // Test 32-bit integer maximums
        test_tokens(
            "+0xFFFF_FFFF",
            &[Token::new(
                0..12,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Isize(0xFFFF_FFFF)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::SignedLarge(0xFFFF_FFFF)),
            )],
        );
        test_tokens(
            "-0xFFFF_FFFF",
            &[Token::new(
                0..12,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-0xFFFF_FFFF)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-0xFFFF_FFFF)),
            )],
        );
        test_tokens(
            "0xFFFF_FFFF",
            &[Token::new(
                0..11,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Usize(0xFFFF_FFFF)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::UnsignedLarge(0xFFFF_FFFF)),
            )],
        );

        // Test 64-bit integer maximums
        #[allow(overflowing_literals)]
        test_tokens(
            "+0xFFFF_FFFF_FFFF_FFFF",
            &[Token::new(
                0..22,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(0xFFFF_FFFF_FFFF_FFFF)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(0xFFFF_FFFF_FFFF_FFFF)),
            )],
        );
        #[allow(overflowing_literals)]
        test_tokens(
            "-0xFFFF_FFFF_FFFF_FFFF",
            &[Token::new(
                0..22,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-0xFFFF_FFFF_FFFF_FFFF)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-0xFFFF_FFFF_FFFF_FFFF)),
            )],
        );
        test_tokens(
            "0xFFFF_FFFF_FFFF_FFFF",
            &[Token::new(
                0..21,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Usize(0xFFFF_FFFF_FFFF_FFFF)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::UnsignedLarge(0xFFFF_FFFF_FFFF_FFFF)),
            )],
        );

        #[cfg(feature = "integer128")]
        {
            #[allow(overflowing_literals)]
            test_tokens(
                "+0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF",
                &[Token::new(
                    0..42,
                    TokenKind::Integer(Integer::SignedLarge(
                        0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF,
                    )),
                )],
            );
            #[allow(overflowing_literals)]
            test_tokens(
                "-0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF",
                &[Token::new(
                    0..42,
                    TokenKind::Integer(Integer::SignedLarge(
                        -0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF,
                    )),
                )],
            );
            test_tokens(
                "0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF",
                &[Token::new(
                    0..41,
                    TokenKind::Integer(Integer::UnsignedLarge(
                        0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF,
                    )),
                )],
            );
        }
    }

    #[test]
    #[allow(overflowing_literals)]
    fn octal_integers() {
        test_tokens(
            "0o1",
            &[Token::new(0..3, TokenKind::Integer(Integer::Usize(1)))],
        );
        test_tokens(
            "0O12",
            &[Token::new(0..4, TokenKind::Integer(Integer::Usize(0o12)))],
        );
        test_tokens(
            "0o12_3",
            &[Token::new(0..6, TokenKind::Integer(Integer::Usize(0o123)))],
        );

        // Test 16-bit integer maximums
        test_tokens(
            "+0o177_777",
            &[Token::new(
                0..10,
                TokenKind::Integer(Integer::Isize(0o177_777)),
            )],
        );
        test_tokens(
            "-0o177_777",
            &[Token::new(
                0..10,
                TokenKind::Integer(Integer::Isize(-0o177_777)),
            )],
        );
        test_tokens(
            "0o177_777",
            &[Token::new(
                0..9,
                TokenKind::Integer(Integer::Usize(0o177_777)),
            )],
        );

        // Test 32-bit integer maximums
        test_tokens(
            "+0o37_777_777_777",
            &[Token::new(
                0..17,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Isize(0o37_777_777_777)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::SignedLarge(0o37_777_777_777)),
            )],
        );
        test_tokens(
            "-0o37_777_777_777",
            &[Token::new(
                0..17,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-0o37_777_777_777)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-0o37_777_777_777)),
            )],
        );
        test_tokens(
            "0o37_777_777_777",
            &[Token::new(
                0..16,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Usize(0o37_777_777_777)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::UnsignedLarge(0o37_777_777_777)),
            )],
        );

        // Test 64-bit integer maximums
        test_tokens(
            "+0o1_777_777_777_777_777_777_777",
            &[Token::new(
                0..32,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(0o1_777_777_777_777_777_777_777)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(0o1_777_777_777_777_777_777_777)),
            )],
        );
        test_tokens(
            "-0o1_777_777_777_777_777_777_777",
            &[Token::new(
                0..32,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-0o1_777_777_777_777_777_777_777)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-0o1_777_777_777_777_777_777_777)),
            )],
        );
        test_tokens(
            "0o1_777_777_777_777_777_777_777",
            &[Token::new(
                0..31,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Usize(0o1_777_777_777_777_777_777_777)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::UnsignedLarge(0o1_777_777_777_777_777_777_777)),
            )],
        );

        #[cfg(feature = "integer128")]
        {
            test_tokens(
                "+0o3_777_777_777_777_777_777_777_777_777_777_777_777_777_777",
                &[Token::new(
                    0..60,
                    TokenKind::Integer(Integer::SignedLarge(
                        0o3_777_777_777_777_777_777_777_777_777_777_777_777_777_777,
                    )),
                )],
            );
            test_tokens(
                "-0o3_777_777_777_777_777_777_777_777_777_777_777_777_777_777",
                &[Token::new(
                    0..60,
                    TokenKind::Integer(Integer::SignedLarge(
                        -0o3_777_777_777_777_777_777_777_777_777_777_777_777_777_777,
                    )),
                )],
            );
            test_tokens(
                "0o3_777_777_777_777_777_777_777_777_777_777_777_777_777_777",
                &[Token::new(
                    0..59,
                    TokenKind::Integer(Integer::UnsignedLarge(
                        0o3_777_777_777_777_777_777_777_777_777_777_777_777_777_777,
                    )),
                )],
            );
        }
    }

    #[test]
    #[allow(overflowing_literals)]
    fn binary_integers() {
        test_tokens(
            "0b1",
            &[Token::new(0..3, TokenKind::Integer(Integer::Usize(1)))],
        );
        test_tokens(
            "0B10",
            &[Token::new(0..4, TokenKind::Integer(Integer::Usize(0b10)))],
        );
        test_tokens(
            "0b10_1",
            &[Token::new(0..6, TokenKind::Integer(Integer::Usize(0b101)))],
        );

        // Test 16-bit integer maximums
        test_tokens(
            "+0b1111_1111_1111_1111",
            &[Token::new(
                0..22,
                TokenKind::Integer(Integer::Isize(0b1111_1111_1111_1111)),
            )],
        );
        test_tokens(
            "-0b1111_1111_1111_1111",
            &[Token::new(
                0..22,
                TokenKind::Integer(Integer::Isize(-0b1111_1111_1111_1111)),
            )],
        );
        test_tokens(
            "0b1111_1111_1111_1111",
            &[Token::new(
                0..21,
                TokenKind::Integer(Integer::Usize(0b1111_1111_1111_1111)),
            )],
        );

        // Test 32-bit integer maximums
        test_tokens(
            "+0b1111_1111_1111_1111_1111_1111_1111_1111",
            &[Token::new(
                0..42,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Isize(0b1111_1111_1111_1111_1111_1111_1111_1111)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::SignedLarge(
                    0b1111_1111_1111_1111_1111_1111_1111_1111,
                )),
            )],
        );
        test_tokens(
            "-0b1111_1111_1111_1111_1111_1111_1111_1111",
            &[Token::new(
                0..42,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-0b1111_1111_1111_1111_1111_1111_1111_1111)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(
                    -0b1111_1111_1111_1111_1111_1111_1111_1111,
                )),
            )],
        );
        test_tokens(
            "0b1111_1111_1111_1111_1111_1111_1111_1111",
            &[Token::new(
                0..41,
                #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
                TokenKind::Integer(Integer::Usize(0b1111_1111_1111_1111_1111_1111_1111_1111)),
                #[cfg(target_pointer_width = "16")]
                TokenKind::Integer(Integer::UnsignedLarge(
                    0b1111_1111_1111_1111_1111_1111_1111_1111,
                )),
            )],
        );

        // Test 64-bit integer maximums
        test_tokens(
            "+0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
            &[Token::new(
                0..82,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111)),
            )],
        );
        test_tokens(
            "-0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
            &[Token::new(
                0..82,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Isize(-0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::SignedLarge(-0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111)),
            )],
        );
        test_tokens(
            "0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
            &[Token::new(
                0..81,
                #[cfg(target_pointer_width = "64")]
                TokenKind::Integer(Integer::Usize(0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111)),
                #[cfg(not(target_pointer_width = "64"))]
                TokenKind::Integer(Integer::UnsignedLarge(0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111)),
            )],
        );

        #[cfg(feature = "integer128")]
        {
            test_tokens(
                "+0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
                &[Token::new(
                    0..162,
                    TokenKind::Integer(Integer::SignedLarge(
                        0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111,
                    )),
                )],
            );
            test_tokens(
                "-0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
                &[Token::new(
                    0..162,
                    TokenKind::Integer(Integer::SignedLarge(
                        -0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111,
                    )),
                )],
            );
            test_tokens(
                "0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
                &[Token::new(
                    0..161,
                    TokenKind::Integer(Integer::UnsignedLarge(
                        0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111,
                    )),
                )],
            );
        }
    }

    #[test]
    fn floats() {
        test_tokens("0.", &[Token::new(0..2, TokenKind::Float(0.))]);
        test_tokens("1.0", &[Token::new(0..3, TokenKind::Float(1.))]);
        test_tokens("-1.0", &[Token::new(0..4, TokenKind::Float(-1.))]);
        test_tokens("+1.0", &[Token::new(0..4, TokenKind::Float(1.))]);
        test_tokens("-1.0e1", &[Token::new(0..6, TokenKind::Float(-10.))]);
        test_tokens("+1.0e1", &[Token::new(0..6, TokenKind::Float(10.))]);
        test_tokens("-1.0e+1", &[Token::new(0..7, TokenKind::Float(-10.))]);
        test_tokens("+1.0e+1", &[Token::new(0..7, TokenKind::Float(10.))]);
        test_tokens("-10.0e-1", &[Token::new(0..8, TokenKind::Float(-1.))]);
        test_tokens("+10.0e-1", &[Token::new(0..8, TokenKind::Float(1.))]);
        test_tokens("-1.0e10", &[Token::new(0..7, TokenKind::Float(-1e10))]);
        test_tokens("+1.0e10", &[Token::new(0..7, TokenKind::Float(1.0e10))]);
        test_tokens("-1e10", &[Token::new(0..5, TokenKind::Float(-1e10))]);
        test_tokens("+1e10", &[Token::new(0..5, TokenKind::Float(1e10))]);
    }

    #[test]
    fn maps() {
        test_tokens(
            "{a:1,b:2}",
            &[
                Token::new(0..1, TokenKind::Open(Balanced::Brace)),
                Token::new(1..2, TokenKind::Identifier(Cow::Borrowed("a"))),
                Token::new(2..3, TokenKind::Colon),
                Token::new(3..4, TokenKind::Integer(Integer::Usize(1))),
                Token::new(4..5, TokenKind::Comma),
                Token::new(5..6, TokenKind::Identifier(Cow::Borrowed("b"))),
                Token::new(6..7, TokenKind::Colon),
                Token::new(7..8, TokenKind::Integer(Integer::Usize(2))),
                Token::new(8..9, TokenKind::Close(Balanced::Brace)),
            ],
        );
    }
}
