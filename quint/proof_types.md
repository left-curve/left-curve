We defined other records, such as `LeafOp` and `InnerOp` a bit differently than Rust implementation. Rust of `LeafOp` is `LeafOp`, and the implementation additionally stores hashing and length functions: `hash`, `prehashBytes`, `prehashBytes`, `len`. Since we fixed the specification to Grug JMT, we do not have to carry them around.

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
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
```bluespec "definitions" +=
type LeafOp = {
  prefix: Term
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
The same applies to `InnerOp`. We don't need to carry `hash` around.

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
We defined Existence and Non Existence proofs. They correspond to the [following Rust structures](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/cosmos.ics23.v1.rs#L24C1-L48C2).

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
type ExistenceProof = {
  key: Bytes, 
  value: Bytes, 
  leaf: LeafOp, 
  path: List[InnerOp]
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// a proof of non-existence of a key
```
-->
```bluespec "definitions" +=
type NonExistenceProof = {
  key: Bytes, 
  left: Option[ExistenceProof], 
  right: Option[ExistenceProof]
}
```

> [!TIP]
> In Rust implementation of ICS23, there is a comment that suggests removing `NonExistenceProof.key` because it is unnecessary. Rust function [`verify_non_existence()`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L34) never uses `proof.key`.