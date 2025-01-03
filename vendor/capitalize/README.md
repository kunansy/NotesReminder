# Capitalize

[![Crate version](https://img.shields.io/crates/v/capitalize)](https://crates.io/crates/capitalize)
[![Unlicense](https://img.shields.io/crates/l/capitalize)](https://unlicense.org/)
[![Crates.io downloads](https://img.shields.io/crates/d/capitalize)](https://crates.io/crates/capitalize)
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/jhg/capitalize-rs/Test%20&%20Lint/main)](https://github.com/jhg/capitalize-rs/actions/workflows/test.yml)

First letter to uppercase, the rest to lowercase. And other common alternatives.

Extensively tested and optimized. Forbidden `unsafe` in the whole crate.

## Examples

```rust
use capitalize::Capitalize;

assert_eq!("hello ✨ world".capitalize(), "Hello ✨ world");
```

Behavior is like [Python's `str.capitalize`], read [`capitalize` reference][Capitalize::capitalize] for details.

## Extensively Tested

Languages tested:

- English
- Spanish
- German
- Turkish
- French
- Russian
- Ukrainian
- Greek
- Chinese
- Arabic
- Hebrew
- Korean
- Japanese
- Thai
- Hindi
- Bulgarian
- Serbian
- Macedonian
- Polish
- Czech
- Slovak
- Croatian
- Icelandic
- Armenian
- Albanian
- Mongolian
- Coptic

[Capitalize::capitalize]: https://docs.rs/capitalize/latest/capitalize/trait.Capitalize.html#tymethod.capitalize
[Python's `str.capitalize`]: https://docs.python.org/3/library/stdtypes.html#str.capitalize
