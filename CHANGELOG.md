# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- markdownlint-disable no-duplicate-heading -->

## Unreleased

### Breaking CHanges

- `parser::Config::allow_implicit_map` has been renamed to
  `allow_implicit_map_at_root`.
- These types are now marked as `#[non_exhaustive]`:
  - `parser::Config`
  - `ser::Config`
  - `writer::Config`

### Fixes

- Raw strings and byte strings without any `#`s can now be used. E.g., `r"\"`
- Implicit map support now supports serializing and deserializing any map-like
  type.

### Added

- When the new flag `ser::Config::anonymous_structs` is enabled, structures will
  be written without their name.


## v0.1.0

Initial release.