# Proof types

This document describes how we defined proof types, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible. We defined types:

- [`LeafOp`](#leafop)
- [`InnerOp`](#innerop)
- [`ExistenceProof`](#existenceproof)
- [`NonExistenceProof`](#nonexistenceproof)
- [`CommitmentProof`](#commitmentproof)
Types used are similar to the original Rust implementation. There are some minor differences, but those will be addressed in detail here. We based our types on [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs) file.

<!-- Boilerplate: tangled from comment to avoid markdown rendering
```bluespec proof_types.qnt
// -*- mode: Bluespec; -*-

module proof_types {
  import basicSpells.* from "./spells/basicSpells"
  import hashes.* from "./hashes"

  <<<definitions>>>
}
```
-->
## LeafOp

We defined record `LeafOp` a bit differently than Rust implementation. Rust implementation of `LeafOp` additionally stores hashing and length functions: `hash`, `prehashBytes`, `prehashBytes`, `len`. Since we fixed the specification to Grug JMT, we do not have to carry them around.

```rust
pub struct LeafOp {
  #[prost(enumeration = "HashOp", tag = "1")]
  pub hash: i32,
  #[prost(enumeration = "HashOp", tag = "2")]
  pub prehash_key: i32,
  #[prost(enumeration = "HashOp", tag = "3")]
  pub prehash_value: i32,
  #[prost(enumeration = "LengthOp", tag = "4")]
  pub length: i32,
  /// prefix is a fixed bytes that may optionally be included at the beginning to differentiate
  /// a leaf node from an inner node.
  #[prost(bytes = "vec", tag = "5")]
  pub prefix: ::prost::alloc::vec::Vec<u8>,
}
```

```bluespec "definitions" +=
type LeafOp = {
  prefix: Term
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
## InnerOp

We also defined record `InnerOp` a bit differently than Rust implementation. Rust implementation of `InnerOp` additionally stores hashing function. Since we fixed the specification to Grug JMT, we do not have to carry them around.

```rust
pub struct InnerOp {
  #[prost(enumeration = "HashOp", tag = "1")]
  pub hash: i32,
  #[prost(bytes = "vec", tag = "2")]
  pub prefix: ::prost::alloc::vec::Vec<u8>,
  #[prost(bytes = "vec", tag = "3")]
  pub suffix: ::prost::alloc::vec::Vec<u8>,
}
```

```bluespec "definitions" +=
type InnerOp = {
  prefix: Term,
  suffix: Term
}
```

<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// a proof of existence of (key, value)
```
-->
## ExistenceProof

We defined ExistenceProof so it corresponds to the [Rust ExistenceProof](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/cosmos.ics23.v1.rs#L24C1-L33C2).

```rust
pub struct ExistenceProof {
    #[prost(bytes = "vec", tag = "1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub leaf: ::core::option::Option<LeafOp>,
    #[prost(message, repeated, tag = "4")]
    pub path: ::prost::alloc::vec::Vec<InnerOp>,
}
```

```bluespec "definitions" +=
type ExistenceProof = {
  key: Bytes,
  value: Bytes,
  leaf: LeafOp,
  path: List[InnerOp]
}
```

> [!TIP]
> `ExistenceProof.leaf` is never used in our specification, but since it is defined in [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs), we decided to keep it and mimic Rust code faithfully.
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// a proof of non-existence of a key
```
-->
## NonExistenceProof

We defined NonExistenceProof so it corresponds to the [Rust NonExistenceProof](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/cosmos.ics23.v1.rs#L40C1-L48C2).

```rust
pub struct NonExistenceProof {
    /// TODO: remove this as unnecessary??? we prove a range
    #[prost(bytes = "vec", tag = "1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub left: ::core::option::Option<ExistenceProof>,
    #[prost(message, optional, tag = "3")]
    pub right: ::core::option::Option<ExistenceProof>,
}
```

```bluespec "definitions" +=
type NonExistenceProof = {
  key: Bytes,
  left: Option[ExistenceProof],
  right: Option[ExistenceProof]
}
```

> [!TIP]
> In Rust implementation of ICS23, there is a comment that suggests removing `NonExistenceProof.key` because it is unnecessary. Rust function [`verify_non_existence()`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L34) never uses `proof.key`.

<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
## CommitmentProof

We defined `CommitmentProof` so it corresponds to the [Rust CommitmentProof](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/cosmos.ics23.v1.rs#L58-L71).

```bluespec "definitions" +=
type CommitmentProof =
  | Exist(ExistenceProof)
  | NonExist(NonExistenceProof)
```

> [!TIP]
> We did not model `Compressed` and `Batch` types of `CommitmentProof`.