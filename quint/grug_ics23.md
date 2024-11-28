# Grug ICS23 proof verification

This document describes how ICS23 proof verification was modelled in Quint, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible.

Most of the correspondance is shown by comparing the Rust code with Quint code short snippets at a time.

This document covers main verification functions:

- [`verifyMembership`](#verifying-existence-proof)
- [`verifyNonMembership`](#verifying-non-existence-proof)

And other helper functions that are used:

- [`verify`](#verify-parts-of-proof)
- [`existsCalculate`](#calculating-hash-from-existence-proof)
- [`hasPadding`](#has_padding)
- [`orderFromPadding`](#order_from_padding)
- [`leftBranchesAreEmpty`](#left_branches_empty)
- [`isLeftMost`](#is_left_most)
- [`rightBranchesAreEmpty`](#right_branches_empty)
- [`isRightMost`](#is_right_most)
- [`isLeftStep`](#is_left_step)
- [`isLeftNeighbor`](#is_left_neighbor)

> [!TIP]
> This markdown file contains some metadata and comments that enable it to be tangled to a full Quint file (using [lmt](https://github.com/driusan/lmt)). The Quint file can be found at [grug_ics23.qnt](./grug_ics23.qnt).

<!-- Boilerplate: tangled from comment to avoid markdown rendering
```bluespec grug_ics24.qnt
// -*- mode: Bluespec; -*-

// This is a protocol specification of ICS23, tuned towards the Grug JMT
// (The original spec was for the IAVL case.)
//
// For details of ICS23, see:
// https://github.com/cosmos/ibc/tree/main/spec/core/ics-023-vector-commitments
//
// For the implementation of ICS23, see:
// https://github.com/cosmos/ics23
//
// We still have to parameterize the spec with the data structure parameters
// such as MinPrefixLen, MaxPrefixLen, ChildSize, and hash.
//
// Igor Konnov, Informal Systems, 2022-2023
// Josef Widder, Informal Systems, 2024
// Aleksandar Ignjatijevic, Informal Systems, 2024

module grug_ics23 {
  import hashes.* from "./hashes"
  import node.* from "./node"
  import basicSpells.* from "./spells/basicSpells"
  import utils.* from "./utils"
  // type aliases for readability
  type Key_t = Bytes_t
  type Value_t = Bytes_t
  type CommitmentRoot_t = Term_t
  type CommitmentProof_t = Term_t

  <<<definitions>>>
}
```
-->
## Types

Types used are similar to the original Rust implementation. There are some minor differences, but those will be addressed in detail here. We based our types on [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs) file.

This specification was inspired by [IAVL Quint specification](https://github.com/informalsystems/quint/blob/c9f8ca04afc3f9a69d46f8423b5b99e6cff25a3c/examples/cosmos/ics23/ics23.qnt). Original specification was tuned to IAVL, which meant that certain parameters had different values, comparing to JMT.
Firstly, we opted to create a record Ics23ProofSpecification that will emulate and simplify the following Rust structure:

```rust
pub struct InnerSpec {
  /// Child order is the ordering of the children node, must count from 0
  /// iavl tree is \[0, 1\] (left then right)
  /// merk is \[0, 2, 1\] (left, right, here)
  #[prost(int32, repeated, tag = "1")]
  pub child_order: ::prost::alloc::vec::Vec<i32>,
  #[prost(int32, tag = "2")]
  pub child_size: i32,
  #[prost(int32, tag = "3")]
  pub min_prefix_length: i32,
  /// the max prefix length must be less than the minimum prefix length + child size
  #[prost(int32, tag = "4")]
  pub max_prefix_length: i32,
  /// empty child is the prehash image that is used when one child is nil (eg. 20 bytes of 0)
  #[prost(bytes = "vec", tag = "5")]
  pub empty_child: ::prost::alloc::vec::Vec<u8>,
  /// hash is the algorithm that must be used for each InnerOp
  #[prost(enumeration = "HashOp", tag = "6")]
  pub hash: i32,
}
```

```bluespec "definitions" +=
type Ics23InnerSpec = {
  MinPrefixLen: int, 
  MaxPrefixLen: int,
  ChildSize: int,
  EmptyChild: Term_t
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
We created a variable with the same values that were set in the [following Rust code](https://github.com/left-curve/left-curve/blob/7a0451dfad14d607722c33bec20ae56dd5c3bffa/grug/jellyfish-merkle/src/ics23.rs#L18-L37):

```rust
pub static ICS23_PROOF_SPEC: LazyLock<ProofSpec> = LazyLock::new(|| ProofSpec {
  leaf_spec: Some(LeafOp {
      hash: HashOp::Sha256.into(),
      prehash_key: HashOp::Sha256.into(),
      prehash_value: HashOp::Sha256.into(),
      length: LengthOp::NoPrefix.into(),
      prefix: LEAF_NODE_HASH_PERFIX.to_vec(),
  }),
  inner_spec: Some(InnerSpec {
      child_order: vec![0, 1],
      child_size: Hash256::LENGTH as _,
      min_prefix_length: INTERNAL_NODE_HASH_PREFIX.len() as _,
      max_prefix_length: INTERNAL_NODE_HASH_PREFIX.len() as _,
      empty_child: Hash256::ZERO.to_vec(),
      hash: HashOp::Sha256.into(),
  }),
  max_depth: 256,
  min_depth: 0,
  prehash_key_before_comparison: true,
});
```

As it can be seen, `Ics23ProofSpec.MinPrefixLen` and `Ics23ProofSpec.MaxPrefixLen` have the value `INTERNAL_NODE_HASH_PREFIX.len()` which is `1`. `Ics23ProofSpec.ChildSize` is `32` because `Hash256::LENGTH` returns `32`, and `Ics23ProofSpec.EmptyChild` is `Hash256_ZERO` to correspond to `Hash256::ZERO.to_vec()`.

```bluespec "definitions" +=
pure val Ics23ProofSpec: Ics23InnerSpec= {
  MinPrefixLen: 1,
  MaxPrefixLen: 1,
  ChildSize: 32,
  EmptyChild: Hash256_ZERO
}
```

`Hash256_ZERO` is defined in [hashes.qnt](./hashes.qnt) as:

```bluespec
val Hash256_ZERO = raw([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
```

> [!TIP]
> Values defined here were previously fixed to needs of IAVL tree.
>
> ```bluespec
>  MinPrefixLength = 4
>  MaxPrefixLength = 12
>  ChildSize = 33 // 32 bytes in SHA256 + 1 byte for the length marker
> ```

We defined other records, such as `LEAF_T` and `INNER_T` a bit differently than Rust implementation. Rust of `LEAF_T` is `LeafOp`, implementation additionally stores hashing and length functions: `hash`, `prehashKey`, `prehashValue`, `len`. Since we fix the specification to JMT, we do not have to carry them around.

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
type LEAF_T = {
  prefix: Term_t
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
type INNER_T = {
  prefix: Term_t,
  suffix: Term_t
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// a proof of existence of (key, value)
```
-->
We defined a Existence and Non Existence proofs:

```bluespec "definitions" +=
type ExistsProof_t = {
  key: Key_t, 
  value: Value_t, 
  leaf: LEAF_T, 
  path: List[INNER_T]
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// a proof of non-existence of a key
```
-->
```bluespec "definitions" +=
type NonExistsProof_t = {
  key: Key_t, left: Option[ExistsProof_t], right: Option[ExistsProof_t]
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
## Calculating root hash from existence proof

This function uses existence proof to calculate root hash out of it. To do so, first it calculates hash of the leaf based on `key` and `value` from the proof. To do so, we are using a specific hash function from Grug JMT implementation.
<!--
```bluespec "definitions" +=
/// calculate a hash from an exists proof
```
-->
```bluespec "definitions" +=
def existsCalculate(p: ExistsProof_t): CommitmentProof_t = 
  val leafHash = hashLeafNode({ key_hash: p.key, value_hash: p.value})    
```

Hashing of the leaf emulates [`apply_leaf`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/ops.rs#L16-L28) Rust function:

```rust
// apply_leaf will take a key, value pair and a LeafOp and return a LeafHash
pub fn apply_leaf<H: HostFunctionsProvider>(
  leaf: &LeafOp,
  key: &[u8],
  value: &[u8],
) -> Result<Hash> {
  let mut hash = leaf.prefix.clone();
  let prekey = prepare_leaf_data::<H>(leaf.prehash_key(), leaf.length(), key)?;
  hash.extend(prekey);
  let preval = prepare_leaf_data::<H>(leaf.prehash_value(), leaf.length(), value)?;
  hash.extend(preval);
  Ok(do_hash::<H>(leaf.hash(), &hash))
}
```

After getting the hash of the leaf, it concatanates hashes of other nodes that are in `path`.
<!--
```bluespec "definitions" +=
  // the inner node nodeHashes are concatenated and hashed upwards
```
-->
```bluespec "definitions" +=
  p.path.foldl(leafHash,
    (child, inner) =>
      termHash(inner.prefix.termConcat(child).termConcat(inner.suffix)))
```
This part emulates the [`apply_inner`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/ops.rs#L8-L14) Rust function:
```rust
pub fn apply_inner<H: HostFunctionsProvider>(inner: &InnerOp, child: &[u8]) -> Result<Hash> {
  ensure!(!child.is_empty(), "Missing child hash");
  let mut image = inner.prefix.clone();
  image.extend(child);
  image.extend(&inner.suffix);
  Ok(do_hash::<H>(inner.hash(), &image))
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

The function `existsCalculate` closely resembles
[`calculate_existence_root_for_spec`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L90-L115) Rust function:

```rust
fn calculate_existence_root_for_spec<H: HostFunctionsProvider>(
    proof: &ics23::ExistenceProof,
    spec: Option<&ics23::ProofSpec>,
) -> Result<CommitmentRoot> {
  ensure!(!proof.key.is_empty(), "Existence proof must have key set");
  ensure!(
      !proof.value.is_empty(),
      "Existence proof must have value set"
  );

  if let Some(leaf_node) = &proof.leaf {
      let mut hash = apply_leaf::<H>(leaf_node, &proof.key, &proof.value)?;
      for step in &proof.path {
          hash = apply_inner::<H>(step, &hash)?;

          if let Some(inner_spec) = spec.and_then(|spec| spec.inner_spec.as_ref()) {
              if hash.len() > inner_spec.child_size as usize && inner_spec.child_size >= 32 {
                  bail!("Invalid inner operation (child_size)")
              }
          }
      }
      Ok(hash)
  } else {
      bail!("No leaf operation set")
  }
}
```

## Verifying NonExistence proof

## verify parts of proof

## calculating_hash_from_existence_proof

## has_padding

## order_from_padding

## left_branches_empty

## is_left_most

## right_branches_empty

## is_right_most

## is_left_step

## is_left_neighbor
