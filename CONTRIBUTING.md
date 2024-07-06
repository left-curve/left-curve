# Contributing Guidelines

Guidelines for contributing code to this repository.

## Formatter

Use _nightly_ toolchain to format your code before pushing:

```bash
cargo +nightly fmt --all
```

An easier way to do this is using the following [just](https://github.com/casey/just) command:

```bash
just fmt
```

Also, add the following config to your VS Code settings, and enable format:

```json
{
  "editor.formatOnSave": true,
  "rust-analyzer.rustfmt.extraArgs": ["+nightly"]
}
```

We use [several rustfmt configurations](./rustfmt.toml) that are not yet available in the stable channel.

Make sure to format macros by hand - rustfmt won't format macros.

## Flat structure

We prefer a _flat structure_ for our crates, meaning there should never be a crate nested inside another crate:

```plain
crates/
└── outer-crate/
    ├── inner-crate/
    │   ├── src/
    │   │   └── lib.rs
    │   └── Cargo.toml
    ├── src/
    │   └── lib.rs
    └── Cargo.toml
```

Nor should a crate contain sub-directories:

```plain
crate-name/
    ├── src/
    │   ├── math/
    │   │   └── mod.rs
    │   └── lib.rs
    └── Cargo.toml
```

If you find a crate needing a subdirectory, it's probably too complex, and should be broken down into multiple crates.

## No submodules

Within a single file, there shouldn't be sub-modules:

```rust
mod display {
    use std::fmt::Display;

    impl Display for MyType {
        // ...
    }
}
```

This means adding an extra 4 spaces of indentation to the code, which is ugly.

If you need to section the code, just add a separator comment instead:

```rust
use std::fmt::Display;

// -------------------------- implement display trait --------------------------

impl Display for MyType {
    // ...
}
```

The only exception to this is tests, which we always use a `tests` (plural, not `test`) submodule:

```rust
// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_work() {
        // ...
    }
}
```

## Trait bounds

### Tightness

When implementing methods that involve generic types, the relevant trait bounds must be as tight as possible. This means if a trait is not required for this implementation, it must not be included in the bound.

Trait bound should be _direct_. See the following example on what this means:

```diff
impl<U, const S: u32> FromStr for Decimal<U, S>
where
    Uint<U>: NumberConst + Number + Display + FromStr + From<u128>,
{
    // ...
}

impl<'de, U, const S: u32> de::Visitor<'de> for DecimalVisitor<U, S>
where
-   Uint<U>: NumberConst + Number + Display + FromStr + From<u128>,
+   Decimal<U, S>: FromStr,
    <Decimal<U, S> as FromStr>::Err: Display,
{
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Decimal::from_str(v).map_err(E::custom)
    }
}
```

Here the two ways of writing the trait bound (in red and green) for `DecimalVisitor` are completely equivalent, because

```rust
Uint<U>: NumberConst + Number + Display + FromStr + From<u128>,
```

implies

```rust
Decimal<U, S>: FromStr,
```

However, the visitor utilizes `Decimal`'s `from_str` method; it doesn't `Uint`'s number or display properties. Therefore, and green trait bound on `Decimal` is _direct_ and preferred, while the red one on `Uint` is _indirect_.

### Formatting

_Always_ use the `where` syntax:

```rust
// ❌ Not this:
fn new_error(msg: impl ToString) -> Error { /* ... */ }

// ❌ Not this:
fn new_error<M: ToString>(msg: M) -> Error { /* ... */ }

// ✅ This:
fn new_error<M>(msg: M) -> Error
where
    M: ToString,
{
    // ...
}
```

This is more verbose, but also more readable (in my opinion).

## Grouping imports

Use a single `use` statement at the beginning of the file to import all necessary dependencies:

```rust
// ❌ Not this:
use crate::{Uint128, Uint256};
use serde::{de, ser};
use std::str::FromStr;

// ✅ This:
use {
  crate::{Uint128, Uint256},
  serde::{de, ser},
  std::str::FromStr,
};
```

## Error messages

Error messages should be lowercase, according to [Rust API guidelines](https://github.com/rust-lang/api-guidelines/blob/master/src/interoperability.md#examples-of-error-messages) (also see [a relevant discussion here](https://github.com/rust-lang/api-guidelines/issues/79)).

```diff
#[derive(Debug, thiserror::Error)]
pub enum StdError {
-   #[error("Division by zero: {a} / 0")]
+   #[error("division by zero: {a} / 0")]
    DivisionByZero { a: String },
}
```

## Comments

Comments should be in Markdown format, with a max width of 80.

This is narrower than the max width for code (100), because to me comments are harder to read if they are too wide.

It's helpful to add the following to VS Code config, so that it shows two rulers, one for comments and one for code:

```json
{
  "editor.rulers": [80, 100]
}
```

Prefer comments to be above a line, instead of trailing a line:

```rust
// ❌ Not this:
let digits = S as u32 - decimal_places; // No overflow because decimal_places < S

// ✅ This:
// No overflow because decimal_places < S
let digits = S as u32 - decimal_places;
```

## Trailing whitespaces

Your code shouldn't have any trailing whitespace. We recommend installing [this VS Code extension](https://marketplace.visualstudio.com/items?itemName=shardulm94.trailing-spaces) which highlights all trailing whitespaces.

The last line of a file should end with a newline character (`\n`) which is [customary for UNIX systems](https://unix.stackexchange.com/questions/18743/whats-the-point-in-adding-a-new-line-to-the-end-of-a-file). This can be automated with the following VS Code config:

```json
{
  "files.insertFinalNewline": true
}
```
