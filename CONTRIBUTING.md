# Contributing Guidelines

Guidelines for contributing code to this repository.

## Formatting

Please use _nightly_ toolchain to format your code before pushing. The easiest way to do this is using the following [just](https://github.com/casey/just) command:

```bash
just fmt
```

We use several `rustfmt` configurations that are not yet available in the stable channel.

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
