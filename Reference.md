# `rsn` Syntax Reference

This document aims to describe the accepted syntax for parsing `rsn`. Currently, it is more of a TODO list than actual documentation.

An `rsn` payload contains a single `Value`, which can be one of these types:

- [Integer](#integer): `123`; `-123_456`; `0x0123_aBc`; `0o123_777`;
  `0b1010_1111`
- [Float](#float): `1.`; `-2_000.123_456`; `1e-2`
- [Boolean](#boolean): `true`; `false`
- [Character](#character): `'a'`; `'\''`
- [Byte](#byte): `b'a'`; `b'\''`
- [String](#string): `"hello, world"`; `r#"raw "strings""#`
- [Byte String](#byte-string): `b"hello, world"`; `br#"raw "strings""#`
- [Map](#map): `{key: "value"}`; `{a: 1, b: true,}`
- [List](#list): `[1, 2, 3]`; `["a", "b",]`
- [Tuple](#tuple): `(1, false)`; `(2, true,)`
- [Identified](#identified): `Name`; `Name { a: 1 }`; `Name(1)`

## Integer

Just like in Rust, integers can be represented in four different
representations: decimal (base 10), binary (base 2), octal (base 8), and
hexadecimal (base 16). Integer parsing ignores underscores (`_`) to allow large
literals to be grouped however the user prefers.

If an integer has no explicit sign, it will be parsed as a `usize`. If the value
overflows, it will be promoted to the "large integer size". If the `integer128`
feature is enabled, the "large integer size" is a `u128`. If the feature is not
enabled, the "large integer size" is a `u64`.

If either a `+` or `-` sign is present, the integer will be parsed as an `isize`
with the appropriate sign applied. If the value overflows, it will be promoted
to the "large integer size". If the `integer128` feature is enabled, the "large
integer size" is a `i128`. If the feature is not enabled, the "large integer
size" is a `i64`.

### Syntax

Integer values always begin with a digit (`0-9`) or a sign (`+`/`-`).

1. If a sign (`+` or `-`) is encountered, the literal is parsed as a signed
   number.
2. At least one digit (`0-9`) must be present.
   1. If the first digit is a `0` and it is followed by an `x` or 'X', parse the
      remainder of this literal as a hexadecimal number.
   2. If the first digit is a `0` and it is followed by a `b` or 'B', parse the
      remainder of this literal as a binary number.
   3. If the first digit is a `0` and it is followed by an `o` or 'O', parse the
      remainder of this literal as an octal number.
3. Continue reading digits (`0-9`) or underscores (`_`) until a non-matching
   character is encountered.
4. If the first non-matching character is either a `.`, `e` or `E`, switch to
   parsing this numerical value as a [float](#float).

#### Hexadecimal Syntax

Hexadecimal values are parsed after encountering `0x` while parsing an
[integer](#integer). After this prefix, parsing is done by reading hexadecimal
digits (`0-9`, `a-f`, `A-F`) or underscores (`_`) until a non-matching character
is encountered.

#### Octal Syntax

Octal values are parsed after encountering `0o` while parsing an
[integer](#integer). After this prefix, parsing is done by reading octal digits
(`0-7`) or underscores (`_`) until a non-matching character is encountered.

#### Binary Syntax

Binary values are parsed after encountering `0b` while parsing an
[integer](#integer). After this prefix, parsing is done by reading binary digits
(`0` or `1`) or underscores (`_`) until a non-matching character is encountered.

## Float

- [x] Tokenizer support
  - [x] `inf`/`NaN` support
- [x] Parser support
- [ ] Deserializer Support
- [ ] Documentation
- [ ] Grammar Spec

## Boolean

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## Character

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## Byte

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## String

- [x] Tokenizer support
  - [x] Support same whitespace rules on raw line ending escaping.
  - [ ] Error-by-default on multiple line ending removal with raw line ending
    escaping, just like rustc, but allow a parsing option that prevents the
    errors.
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## Byte String

- [x] Tokenizer support
  - [ ] `b64` prefixed base64-encoded byte strings
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## Map

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## List

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## Tuple

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation

## Identified

- [x] Tokenizer support
- [x] Parser support
- [x] Deserializer Support
- [ ] Documentation
