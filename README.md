# Rsn - Rusty Notation

**This crate is very early in development. Please report any issues [on our
GitHub](https://github.com/khonsulabs/rsn).**

![rsn forbids unsafe code](https://img.shields.io/badge/unsafe-forbid-success)
![rsn is considered alpha](https://img.shields.io/badge/status-alpha-orange)
[![crate version](https://img.shields.io/crates/v/rsn.svg)](https://crates.io/crates/rsn)
[![Live Build Status](https://img.shields.io/github/actions/workflow/status/khonsulabs/rsn/rust.yml?branch=main)](https://github.com/khonsulabs/rsn/actions?query=workflow:Tests)
[![HTML Coverage Report for `main`](https://khonsulabs.github.io/rsn/coverage/badge.svg)](https://khonsulabs.github.io/rsn/coverage/)
[![Documentation for `main`](https://img.shields.io/badge/docs-main-informational)](https://khonsulabs.github.io/rsn/main/rsn/)

A UTF-8 based text format that looks very similar to valid Rust code. This format adheres closely to [Rust's lexical rules][rust-lexer]

## `no_std` support

This crate supports `no_std` targets that support the `alloc` crate.

## Data Types

```rsn
ExampleStruct {
  integers: [42, 0xFF, 0o77, 0b101],
  floats: [42., 3.14, 1e10],
  bools: [true, false],
  chars: ['a', '\''],
  string: "Hello, World!",
  raw_string: r#"I said, "Hello, World!""#,
  bytes: [b'a', b'\''],
  byte_string: b"Hello, World!",
  raw_byte_string: br#"I said, "Hello, World!""#,
  named_map: StructLike {
    field: 42,
  },
  named_tuple: TupleLike(42),
  r#raw_identifiers: true,
  array: [1, 2, 3],
  tuple: (1, 2, 3),
  map: {
    "a": 1,
    "b": 2,
  },
}
```

- Integers (`42`, `0xFF`, `0o77`, `0b101`)
- Floats (`42.`, `3.14`)
- Bool (`true`, `false`)
- Character (`'a'`, `'\''`)
- Byte (`b'a'`, `b'\''`)
- String (`"hello, world"`)
- Raw Strings (`r#"They said, "Hello World!""#`)
- Byte Strings (`b"hello, world"`)
- Named
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

## Differences between Rust syntax and Rsn

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

[rust-lexer]: https://doc.rust-lang.org/reference/lexical-structure.html
