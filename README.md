# rsn - Rust Notation

**This crate is very early in development and is not ready for consumption.**

A UTF-8 based text format that looks very similar to valid Rust code.

The syntax differs from valid Rust code for:

- Map literals. Rust has no syntax for map literals.
- Enum Variants being used without the type name -- `Red` vs `Color::Red`
  - This is technically valid Rust syntax if `use Color::*` is present.
- Infinity and Not-A-Number floats are represented as
  `+inf`/`-inf`/`+NaN`/`-NaN`.
  - For compatibility with Rust syntax, support for
    [`f64::INFINITY`](https://github.com/khonsulabs/rsn/issues/3) is being
    considered.

The rules for parsing literals should match Rust's rules as closely as possible.

## `no_std` support

This crate supports `no_std` targets that support the `alloc` crate.

## Data Types

- Integers (`42`, `0xFF`, `0o77`, `0b101`)
- Floats (`42.`, `3.14`, `)
- Bool (`true`, `false`)
- Character (`'a'`, `'\''`)
- Byte (`b'a'`, `b'\''`)
- String (`"hello, world"`)
- Raw Strings (`r#"They said, "Hello World!""#`)
- Byte Strings (`b"hello, world"`)
- Struct
  - Ident or Raw Ident (`r#foo`)
  - Map or Tuple
- Map
  - List of `<Value>: <Value>` pairs, delimited by comma
  - Trailing comma is optional
- Tuple (empty tuple = Unit)
  - List of `<Value>`s, delimited by comma
  - Trailing comma is optional
- Array
  - List of `<Value>`s, delimited by comma
  - Trailing comma is optional
- Comments `//` and `/* */`

- Potential Extensions via #[] syntax
  - Semi-strict comma-delimited list
  - `#[foo(...), bar = ...,]`
  - All braces/brackets/parens must be paired correctly?

## Other related projects

- [`rsn-fmt`](https://github.com/ModProg/rsn-fmt): A formatter project for `rsn`.
- [`rsn.vim`](https://github.com/ModProg/rsn.vim): A plugin for Vim/NeoVim.

## Why not Ron?

[Ron](https://crates.io/crates/ron) is a great format. There were a few design
decisions that led to this very-similar-yet-not-the-same format being invented:

- `ron` differentiates between Tuples and Lists, while `rsn` treats all
  sequences the same.
- `ron` uses a different syntax for structures and maps. `rsn` uses the same
  syntax for both concepts.
- `ron` has special support for `Option<T>`. `rsn` treats `Option<T>` like any
  other enum.
- `ron`'s parsing rules are close but not the same as Rust, while `rsn` attempts
  to match implementations:
  - Unicode white space and idents (added in
    [ron-rs/ron#444](https://github.com/ron-rs/ron/pull/444))
  - Rust allows `_` in float literals
  - Rust allows for raw line endings to be escaped in string literals.
  - Rust supports byte strings and byte literals, while Ron elected to use
    `base64` encoded strings for byte values.
