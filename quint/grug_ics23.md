# Grug ICS23 proof verification

This document describes how ICS23 proof verification was modelled in Quint, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible.

Most of the correspondance is shown by comparing the Rust code with Quint code short snippets at a time.

This document covers main verification functions:

- [`verifyMembership`](#verifying-membership-proof)
- [`verifyNonMembership`](#verifying-nonmembership-proof)

And other helper functions that are used:

- [`verify`](#verifying-existence)
- [`exists_calculate`](#calculating-root-hash-from-existence-proof)
- [`get_padding`](#get-padding)
- [`is_left_most`](#is-left-most)
- [`is_right_most`](#is-right-most)
- [`has_padding`](#has-padding)
- [`order_from_padding`](#order-from-padding)
- [`left_branches_are_empty`](#left-branches-empty)
- [`right_branches_are_empty`](#right-branches-empty)
- [`is_left_step`](#is-left-step)
- [`is_left_neighbor`](#is-left-neighbor)

> [!TIP]
> This markdown file contains some metadata and comments that enable it to be tangled to a full Quint file (using [lmt](https://github.com/driusan/lmt)). The Quint file can be found at [grug_ics23.qnt](./grug_ics23.qnt).

<!-- Boilerplate: tangled from comment to avoid markdown rendering
```bluespec grug_ics23.qnt
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
// such as min_prefix_length, max_prefix_length, child_size, and hash.
//
// Igor Konnov, Informal Systems, 2022-2023
// Josef Widder, Informal Systems, 2024
// Aleksandar Ignjatijevic, Informal Systems, 2024
// Gabriela Moreira, Informal Systems, 2024

module grug_ics23 {
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import hashes.* from "./hashes"
  import node.* from "./node"
  import proof_types.* from "./proof_types"
  import utils.* from "./utils"

  <<<definitions>>>
}
```
-->
## Types

Types used are similar to the original Rust implementation. There are some minor differences, but those will be addressed in detail here. We based our types on [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs) file.

This specification was inspired by [an existing ICS23 Quint specification](https://github.com/informalsystems/quint/blob/c9f8ca04afc3f9a69d46f8423b5b99e6cff25a3c/examples/cosmos/ics23/ics23.qnt). The original specification was tuned to IAVL, which meant that certain parameters had different values, comparing to Grug JMT.
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
type InnerSpec = {
  min_prefix_length: int, 
  max_prefix_length: int,
  child_size: int,
  empty_child: Term,
  child_order: List[int],
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
      prefix: LeafOp_NODE_HASH_PERFIX.to_vec(),
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

As it can be seen, `Ics23ProofSpec.min_prefix_length` and `Ics23ProofSpec.max_prefix_length` have the value `INTERNAL_NODE_HASH_PREFIX.len()` which is `1`. `Ics23ProofSpec.child_size` is `32` because `Hash256::LENGTH` returns `32`, and `Ics23ProofSpec.empty_child` is `Hash256_ZERO` to correspond to `Hash256::ZERO.to_vec()`.

```bluespec "definitions" +=
pure val ics23::InnerSpec: InnerSpec = {
  min_prefix_length: 1,
  max_prefix_length: 1,
  child_size: 32,
  empty_child: Hash256_ZERO,
  child_order: [0, 1]
}
```

`InnerSpec.child_order` for Grug JMT is defined [here](https://github.com/left-curve/left-curve/blob/4890cb58a4b60acfacbc19d6c041f72db2b5b8ee/grug/jellyfish-merkle/src/ics23.rs#L27).
`Hash256_ZERO` is placed in [hashes.qnt](./hashes.qnt).

```bluespec
val Hash256_ZERO = raw([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
```

> [!TIP]
> Parameters defined in [an existing ICS23 Quint specification](https://github.com/informalsystems/quint/blob/c9f8ca04afc3f9a69d46f8423b5b99e6cff25a3c/examples/cosmos/ics23/ics23.qnt) are fixed to the IAVL tree. In that specification the following parameters were used:
>
> ```bluespec
>  MinPrefixLength = 4
>  MaxPrefixLength = 12
>  ChildSize = 33 // 32 bytes in SHA256 + 1 byte for the length marker
> ```
>
> They can be found [here](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/api.rs#L197-L221).
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
We defined `Padding` as it was defined in Rust:

```rust
struct Padding {
    min_prefix: usize,
    max_prefix: usize,
    suffix: usize,
}
```

```bluespec "definitions" +=
type Padding = {
  min_prefix: int,
  max_prefix: int,
  suffix: int,
}
```

<!--
```bluespec "definitions" +=

/// VerifyMembership returns true iff
/// proof is an ExistenceProof for the given key and value AND
/// calculating the root for the ExistenceProof matches
/// the provided CommitmentRoot
```
-->

## Verifying Membership Proof

`verifyMembership` returns true iff proof is an ExistenceProof for the given key and value AND calculating the root for the ExistenceProof matches the provided CommitmentRoot.
Our `verifyMembership` function emulates [`verify_membership`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/api.rs#L16-L42) Rust function.

```rust
pub fn verify_membership<H: HostFunctionsProvider>(
    proof: &ics23::CommitmentProof,
    spec: &ics23::ProofSpec,
    root: &CommitmentRoot,
    key: &[u8],
    value: &[u8],
) -> bool {
    // ugly attempt to conditionally decompress...
    let mut proof = proof;
    let my_proof;
    if is_compressed(proof) {
        if let Ok(p) = decompress(proof) {
            my_proof = p;
            proof = &my_proof;
        } else {
            return false;
        }
    }

    //    if let Some(ics23::commitment_proof::Proof::Exist(ex)) = &proof.proof {
    if let Some(ex) = get_exist_proof(proof, key) {
        let valid = verify_existence::<H>(ex, spec, root, key, value);
        valid.is_ok()
    } else {
        false
    }
}
```

We did not specify decompressing and CommitmentProof_Batches and we are just focusing on verifying existence.

```bluespec "definitions" +=
def verifyMembership(root: Term, proof: ExistenceProof, key: Bytes, value: Bytes): bool = {
  // TODO: specify Decompress
  // TODO: specify the case of CommitmentProof_Batch
  // TODO: CheckAgainstSpec ensures that the proof can be verified
  //       by the spec checker
  verify(proof, root, key, value)
}
```

## Verifying existence

Verifying membership emulates the [`verify_existence`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L18C1-L32C2) Rust function.

```rust
pub fn verify_existence<H: HostFunctionsProvider>(
    proof: &ics23::ExistenceProof,
    spec: &ics23::ProofSpec,
    root: &[u8],
    key: &[u8],
    value: &[u8],
) -> Result<()> {
    check_existence_spec(proof, spec)?;
    ensure!(proof.key == key, "Provided key doesn't match proof");
    ensure!(proof.value == value, "Provided value doesn't match proof");

    let calc = calculate_existence_root_for_spec::<H>(proof, Some(spec))?;
    ensure!(calc == root, "Root hash doesn't match");
    Ok(())
}
```

Our implementations assumes that proof and spec are alligned and therefore we did not implement `check_existence_spec` function.
<!--
```bluespec "definitions" +=

/// verify that a proof matches a root
```
-->
```bluespec "definitions" +=
def verify(proof, root, key, value) = and {
  key == proof.key,
  value == proof.value,
  root == exists_calculate(proof)
}
```

## Calculating root hash from existence proof

This function uses existence proof to calculate root hash out of it.
<!--
```bluespec "definitions" +=

/// calculate a hash from an exists proof
```
-->
```bluespec "definitions" +=
def exists_calculate(p: ExistenceProof): Term =
```

The function `exists_calculate` emulates
[`calculate_existence_root_for_spec`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L90-L115) Rust function.

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

First it calculates hash of the leaf based on `key` and `value` from the proof. To do so, we are using a specific hash function from Grug JMT implementation.
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

```bluespec "definitions" +=
  val leafHash = hashLeafNode({ key_hash: p.key, value_hash: p.value})    
```

After getting the hash of the leaf, it concatanates hashes of other nodes that are in `path`. This part emulates the [`apply_inner`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/ops.rs#L8-L14) Rust function.

```rust
pub fn apply_inner<H: HostFunctionsProvider>(inner: &InnerOp, child: &[u8]) -> Result<Hash> {
  ensure!(!child.is_empty(), "Missing child hash");
  let mut image = inner.prefix.clone();
  image.extend(child);
  image.extend(&inner.suffix);
  Ok(do_hash::<H>(inner.hash(), &image))
}
```
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

<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// VerifyNonMembership returns true iff
/// proof is (contains) a NonExistenceProof,
/// both left and right sub-proofs are valid existence proofs (see above) or nil,
/// left and right proofs are neighbors (or left/right most if one is nil),
/// provided key is between the keys of the two proofs
```
-->
## Verifying NonMembership proof

`verifyNonMembership` returns true iff proof is a NonExistenceProof, both left and right sub-proofs are valid existence proofs or nil, left and right proofs are neighbors (or left/right most if one is nil), provided key is between the keys of the two proofs. We did not specify decompressing and CommitmentProof_Batches and we are just focusing on verifying existence.
The function `verifyNonMembership` emulates [`verify_non_membership`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L34-L79) Rust function.

```rust
pub fn verify_non_existence<H: HostFunctionsProvider>(
    proof: &ics23::NonExistenceProof,
    spec: &ics23::ProofSpec,
    root: &[u8],
    key: &[u8],
) -> Result<()> {
```

```bluespec "definitions" +=
def verifyNonMembership(root: Term, np: NonExistenceProof, key: Bytes): bool = and {
```

First we check if both `np.left == np.right == None`. This is our way of emulating the following part of the Rust code. This way we are assured that either `np.left` or `np.right` will have some value, and later when we check if `np.left == None`, we are certain that `np.right != None`, and vice versa.

```rust
  if let Some(inner) = &spec.inner_spec {
      match (&proof.left, &proof.right) {
          ...
          (None, None) => bail!("neither left nor right proof defined"),
      }
  }
```

```bluespec "definitions" +=
  np.left != None or np.right != None,
```

After that, we check if `np.left == None` or we can unwrap it and verify it. In Quint specification we are using already hashed keys, and to compare keys we are using Quint function `lessThan()` in which we can safely pass `np.left.unwrap().key` and `key`. This means that we did not have to emulate Rust's `key_for_comparison` that either hashes key if it is not hashed, or returns the already hashed key.

Here is the snippet from the Rust implementation.

```rust
  if let Some(left) = &proof.left {
      verify_existence::<H>(left, spec, root, &left.key, &left.value)?;
      ensure!(
          key_for_comparison(key) > key_for_comparison(&left.key),
          "left key isn't before key"
      );
  }
```

```bluespec "definitions" +=
  np.left == None or and {
    verify(np.left.unwrap(), root, np.left.unwrap().key, np.left.unwrap().value), 
    lessThan(np.left.unwrap().key, key),
  },
```

The same is done for `np.right`. Here is the snippet from the Rust implementation we emulated.

```rust
  if let Some(right) = &proof.right {
      verify_existence::<H>(right, spec, root, &right.key, &right.value)?;
      ensure!(
          key_for_comparison(key) < key_for_comparison(&right.key),
          "right key isn't after key"
      );
  }
```

```bluespec "definitions" +=
  np.right == None or and {
    verify(np.right.unwrap(), root, np.right.unwrap().key, np.right.unwrap().value),
    lessThan(key, np.right.unwrap().key),
  },
```

Since Quint's matching is not as powerful as Rust's, we had to find a work-around solution. Utilizing previously placed checks, we emulated Rust `match` statement in the following way. Here is the snippet from the Rust implementation.

```rust
  if let Some(inner) = &spec.inner_spec {
    match (&proof.left, &proof.right) {
        (Some(left), None) => ensure_right_most(inner, &left.path),
        (None, Some(right)) => ensure_left_most(inner, &right.path),
        (Some(left), Some(right)) => ensure_left_neighbor(inner, &left.path, &right.path),
        ...
    }
  } 
```

```bluespec "definitions" +=
  pure val spec = ics23::InnerSpec
  if (np.left == None) {
    is_left_most(spec, np.right.unwrap().path)
  } else if (np.right == None) {
    is_right_most(spec, np.left.unwrap().path)
  } else {
    is_left_neighbor(spec, np.left.unwrap().path, np.right.unwrap().path)
  }
}
```

Here is the full `verify_non_existence` Rust implementation.

```rust
pub fn verify_non_existence<H: HostFunctionsProvider>(
    proof: &ics23::NonExistenceProof,
    spec: &ics23::ProofSpec,
    root: &[u8],
    key: &[u8],
) -> Result<()> {
    let key_for_comparison = |key: &[u8]| -> Vec<u8> {
        match spec.prehash_key_before_comparison {
            true => do_hash::<H>(
                spec.leaf_spec
                    .as_ref()
                    .map(Cow::Borrowed)
                    .unwrap_or_default()
                    .prehash_key(),
                key,
            ),
            false => key.to_vec(),
        }
    };

    if let Some(left) = &proof.left {
        verify_existence::<H>(left, spec, root, &left.key, &left.value)?;
        ensure!(
            key_for_comparison(key) > key_for_comparison(&left.key),
            "left key isn't before key"
        );
    }
    if let Some(right) = &proof.right {
        verify_existence::<H>(right, spec, root, &right.key, &right.value)?;
        ensure!(
            key_for_comparison(key) < key_for_comparison(&right.key),
            "right key isn't after key"
        );
    }

    if let Some(inner) = &spec.inner_spec {
        match (&proof.left, &proof.right) {
            (Some(left), None) => ensure_right_most(inner, &left.path),
            (None, Some(right)) => ensure_left_most(inner, &right.path),
            (Some(left), Some(right)) => ensure_left_neighbor(inner, &left.path, &right.path),
            (None, None) => bail!("neither left nor right proof defined"),
        }
    } else {
        bail!("Inner Spec missing")
    }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

## Get Padding

We defined `get_padding` in the similar way as it was defined in the [Rust implementation](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L305-L317)

```rust
fn get_padding(spec: &ics23::InnerSpec, branch: i32) -> Result<Padding> {
  if let Some(&idx) = spec.child_order.iter().find(|&&x| x == branch) {
      let prefix = idx * spec.child_size;
      let suffix = spec.child_size as usize * (spec.child_order.len() - 1 - idx as usize);
      Ok(Padding {
          min_prefix: (prefix + spec.min_prefix_length) as usize,
          max_prefix: (prefix + spec.max_prefix_length) as usize,
          suffix,
      })
  } else {
      bail!("Branch {} not found", branch);
  }
}
```

We used `None` to emulate `bail!()` in Rust implementation.

```bluespec "definitions" +=
pure def get_padding(spec: InnerSpec, branch: int): Option[Padding] = {
  match spec.child_order.findFirst(x => x == branch) {
    | Some(idx) => {
      pure val prefix = idx * spec.child_size
      pure val suffix = spec.child_size * (spec.child_order.length() - 1 - idx)
      Some({
        min_prefix: prefix + spec.min_prefix_length,
        max_prefix: prefix + spec.max_prefix_length,
        suffix: suffix,
      })
    }
    | None => None
  }
}
```

Based on the `branch` value, which will be either `0` or `1`.

- In the case when `branch = 0`, padding will look like this:
  
  ```bluespec
  {
    min_prefix: spec.min_prefix_length,
    max_prefix: spec.max_prefix_length,
    suffix: spec.child_size,
  }
  ```

- In the case when `branch = 1`, padding will look like this:
  
  ```bluepsec
  {
    min_prefix: spec.min_prefix_length + spec.child_size,
    max_prefix: spec.max_prefix_length + spec.child_size,
    suffix: 0,
  }
  ```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// is_left_most returns true if this is the left-most path in the tree,
/// excluding placeholder (empty child) nodes
```
-->
## Is Left Most

This function returns true if this is the left-most path in the tree, excluding placeholder (empty child) nodes. This function emulates the [`ensure_left_most`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L222C1-L232C2) Rust function.

```rust
// ensure_left_most fails unless this is the left-most path in the tree, excluding placeholder (empty child) nodes
fn ensure_left_most(spec: &ics23::InnerSpec, path: &[ics23::InnerOp]) -> Result<()> {
    let pad = get_padding(spec, 0)?;
    // ensure every step has a prefix and suffix defined to be leftmost, unless it is a placeholder node
    for step in path {
        if !has_padding(step, &pad) && !left_branches_are_empty(spec, step)? {
            bail!("step not leftmost")
        }
    }
    Ok(())
}
```

```bluespec "definitions" +=
def is_left_most(spec: InnerSpec, path: List[InnerOp]): bool = {
  match get_padding(spec, 0) {
    | Some(pad) => {
      path.indices().forall(i =>
        val step = path[i]
        or {
          // the path goes left
          has_padding(step, pad),
          // the path goes right, but the left child is empty (a gap)
          left_branches_are_empty(spec, step)
        }
      )
    }
    | None => false
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// is_right_most returns true if this is the left-most path in the tree,
/// excluding placeholder (empty child) nodes
```
-->
## Is Right Most

`is_right_most` function performs in the same way as `is_left_most`, with only difference being the `get_padding()` is called with `branch = 1`.
This function emulates the [`ensure_right_most`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L234-L245) Rust function. Essentially, these two functions perform the same way, only differences revolving around Quint's inability to early return in the event of error.

```rust
fn ensure_right_most(spec: &ics23::InnerSpec, path: &[ics23::InnerOp]) -> Result<()> {
  let idx = spec.child_order.len() - 1;
  let pad = get_padding(spec, idx as i32)?;
  // ensure every step has a prefix and suffix defined to be rightmost, unless it is a placeholder node
  for step in path {
      if !has_padding(step, &pad) && !right_branches_are_empty(spec, step)? {
          bail!("step not leftmost")
      }
  }
  Ok(())
}
```

```bluespec "definitions" +=
def is_right_most(spec: InnerSpec, path: List[InnerOp]): bool = {
  pure val idx = spec.child_order.length() - 1
  match get_padding(spec, idx) {
    | Some(pad) => {
      path.indices().forall(i =>
        val step = path[i]
        or {
          // the path goes right
          has_padding(step, pad),
          // the path goes left, but the right child is empty (a gap)
          right_branches_are_empty(spec, step)
        }
      )
    }
    | None => false
  }
}
```

<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// checks if an op has the expected padding
```
-->

## Has Padding

`has_padding` Quint function emulates [`has_padding`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L299C1-L303C2) Rust function.

```rust
fn has_padding(op: &ics23::InnerOp, pad: &Padding) -> bool {
  (op.prefix.len() >= pad.min_prefix)
      && (op.prefix.len() <= pad.max_prefix)
      && (op.suffix.len() == pad.suffix)
}
```

For getting the length of prefix and suffix we are using `termLen` function, which is defined in [hashes.qnt](./hashes.qnt)

```bluespec "definitions" +=
def has_padding(op: InnerOp, pad: Padding): bool = and {
  op.prefix.termLen() >= pad.min_prefix,
  op.prefix.termLen() <= pad.max_prefix,
  // When inner turns left, suffixLen == child_size,
  // that is, we store the hash of the right child in the suffix.
  // When inner turns right, suffixLen == 0,
  // that is, we store the hash of the left child in the prefix.
  op.suffix.termLen() == pad.suffix,
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// This will look at the proof and determine which order it is.
/// So we can see if it is branch 0, 1, 2 etc... to determine neighbors
```
-->
## Order from padding

`order_from_padding` Quint function emulates [`order_from_padding`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L282-L291) Rust function. This function will take the proof and determine which order it is, so we can see if it is branch 0, 1 to determine neighbors.

```rust
fn order_from_padding(spec: &ics23::InnerSpec, op: &ics23::InnerOp) -> Result<i32> {
  let len = spec.child_order.len() as i32;
  for branch in 0..len {
    let padding = get_padding(spec, branch)?;
    if has_padding(op, &padding) {
        return Ok(branch);
    }
  }
  bail!("padding doesn't match any branch");
}
```

We are using `find_first()` to simulate early return `return Ok(branch)` in Rust implementation. It will return `branch` for which `has_padding()`, called with parameter `padding` gotten for said `branch`, returns `true`.
Since we know that `spec.child_order.length() == 2`, algorithm will never _not_ return padding when called `get_padding()`.

```bluespec "definitions" +=
def order_from_padding(spec: InnerSpec, op: InnerOp): Option[int] = {
  pure val len = spec.child_order.length()
  range(0, len).find_first(branch => {
    match get_padding(spec, branch) {
      | Some(padding) => has_padding(op, padding)
      | None => false // This should actually early return but this is impossible for our InnerSpec
    }
  })
}
```

<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// left_branches_are_empty returns true if the padding bytes correspond to all
/// empty siblings on the left side of a branch, ie. it's a valid placeholder
/// on a leftmost path
```
-->
## Left Branches Empty

`left_branches_are_empty` Quint function returns true if the padding bytes correspond to all empty siblings on the left side of a branch, ie. it's a valid placeholder on a leftmost path.
It emulates [`left_branches_are_empty`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L321-L343) Rust function.
The signature of Rust and Quint functions:

```rust
fn left_branches_are_empty(spec: &ics23::InnerSpec, op: &ics23::InnerOp) -> Result<bool> {
```

```bluespec "definitions" +=
def left_branches_are_empty(spec: InnerSpec, op: InnerOp): bool = and {
```

Firstly, we get order from padding, and then we check if there is and index found and if it is not `0`. This corresponds to the following Rust code:

```rust
let idx = order_from_padding(spec, op)?;
// count branches to left of this
let left_branches = idx as usize;
if left_branches == 0 {
    return Ok(false);
}
```

```bluespec "definitions" +=
  pure val idx = order_from_padding(spec, op)
  pure val left_branches = idx.unwrap()
  if (left_branches == 0) {
    false
  }
```

If `left_branches != 0`, then we can continue:

```rust
  let child_size = spec.child_size as usize;
  // compare prefix with the expected number of empty branches
  let actual_prefix = match op.prefix.len().checked_sub(left_branches * child_size) {
      Some(n) => n,
      _ => return Ok(false),
  };
  for i in 0..left_branches {
      let idx = spec.child_order.iter().find(|&&x| x == i as i32).unwrap();
      let idx = *idx as usize;
      let from = actual_prefix + idx * child_size;
      if spec.empty_child != op.prefix[from..from + child_size] {
          return Ok(false);
      }
  }
  Ok(true)
```

We have implemented a custom `checked_sub` in [utils.qnt](./utils.qnt) which emulates Rust `checked_sub` function.

```bluespec
pure def checked_sub(a: int, b: int): Option[int] = {
  if (b > a) {
    None
  } else {
    Some(a - b)
  }
}
```

Because Quint does not support early return, we had to `match` the outcome of `checked_sub` function.

```bluespec "definitions" +=
  else {
    pure val child_size = spec.child_size
    match op.prefix.termLen().checked_sub(left_branches * child_size) {
      | Some(actual_prefix) => {
          0.to(left_branches).forall(i => {
            pure val idx = spec.child_order.findFirst(x => x == i).unwrap()
            val from_index = actual_prefix + idx * child_size
            spec.empty_child == op.prefix.termSlice(from_index, from_index + child_size)
          })
        }
      | None => false
    }
  }
}
```

Here is the full Rust code of `left_branches_are_empty` function.

```rust
fn left_branches_are_empty(spec: &ics23::InnerSpec, op: &ics23::InnerOp) -> Result<bool> {
  let idx = order_from_padding(spec, op)?;
  // count branches to left of this
  let left_branches = idx as usize;
  if left_branches == 0 {
      return Ok(false);
  }
  let child_size = spec.child_size as usize;
  // compare prefix with the expected number of empty branches
  let actual_prefix = match op.prefix.len().checked_sub(left_branches * child_size) {
      Some(n) => n,
      _ => return Ok(false),
  };
  for i in 0..left_branches {
      let idx = spec.child_order.iter().find(|&&x| x == i as i32).unwrap();
      let idx = *idx as usize;
      let from = actual_prefix + idx * child_size;
      if spec.empty_child != op.prefix[from..from + child_size] {
          return Ok(false);
      }
  }
  Ok(true)
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// right_branches_are_empty returns true if the padding bytes correspond
/// to all empty siblings on the right side of a branch,
/// i.e. it's a valid placeholder on a rightmost path
```
-->
## Right Branches Empty

`right_branches_are_empty` Quint function returns true if the padding bytes correspond to all empty siblings on the right side of a branch, i.e. it's a valid placeholder on a rightmost path.
It emulates [`right_branches_are_empty`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L345-L367) Rust function.
The signature of Rust and Quint functions:

```rust
fn right_branches_are_empty(spec: &ics23::InnerSpec, op: &ics23::InnerOp) -> Result<bool> {
```

```bluespec "definitions" +=
def right_branches_are_empty(spec: InnerSpec, op: InnerOp): bool = {
```

Firstly, we get order from padding, and then we check if there is and index found and if it is not `0`. This corresponds to the following Rust code:

```rust
let idx = order_from_padding(spec, op)?;
```

```bluespec "definitions" +=
  pure val idx = order_from_padding(spec, op)
```

Since in the event of an error in `order_from_padding()` `right_branches_are_empty()` will terminate, we wrapped outcome in one big `and` that will assure `idx != None`, before unwrapping it.

```rust
for i in 0..right_branches {
  let idx = spec.child_order.iter().find(|&&x| x == i as i32).unwrap();
  let idx = *idx as usize;
  let from = idx * spec.child_size as usize;
  if spec.empty_child != op.suffix[from..from + spec.child_size as usize] {
      return Ok(false);
  }
}
```

```bluespec "definitions" +=
  and {
    idx != None,
    // count branches to right of this one
    pure val right_branches = spec.child_order.length() - 1 - idx.unwrap()
    // compare suffix with the expected number of empty branches
    if (right_branches == 0) {
      false
    } else if (op.suffix.termLen() != spec.child_size) {
      false
    } else {
      0.to(right_branches).forall(i => {
        pure val idx = spec.child_order.findFirst(x => x == i).unwrap()
        val from_index = idx * spec.child_size
        spec.empty_child == op.suffix.termSlice(from_index, from_index + spec.child_size)
      })
    }
  }
}
```

Here is the full `right_branches_are_empty` Rust function:

```rust
fn right_branches_are_empty(spec: &ics23::InnerSpec, op: &ics23::InnerOp) -> Result<bool> {
  let idx = order_from_padding(spec, op)?;
  // count branches to right of this one
  let right_branches = spec.child_order.len() - 1 - idx as usize;
  // compare suffix with the expected number of empty branches
  if right_branches == 0 {
      return Ok(false);
  }
  if op.suffix.len() != spec.child_size as usize {
      return Ok(false);
  }
  for i in 0..right_branches {
      let idx = spec.child_order.iter().find(|&&x| x == i as i32).unwrap();
      let idx = *idx as usize;
      let from = idx * spec.child_size as usize;
      if spec.empty_child != op.suffix[from..from + spec.child_size as usize] {
          return Ok(false);
      }
  }
  Ok(true)
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// is_left_step assumes left and right have common parents
/// checks if left is exactly one slot to the left of right
```
-->
## Is Left Step

`isLeftStep` function assumes `left` and `right` parameters have common parent and checks if `left` is exactly one slot to the left of the `right`. This function emulates [`is_left_step`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L272C1-L280C2) Rust function.

```rust
fn is_left_step(
    spec: &ics23::InnerSpec,
    left: &ics23::InnerOp,
    right: &ics23::InnerOp,
) -> Result<bool> {
  let left_idx = order_from_padding(spec, left)?;
  let right_idx = order_from_padding(spec, right)?;
  Ok(left_idx + 1 == right_idx)
}
```

Again, because of Quint not supporting early returns, we used `and` to assure `left_idx` and `right_idx` will not be `None`.

```bluespec "definitions" +=
def is_left_step(spec: InnerSpec, left: InnerOp, right: InnerOp): bool = {
  // 'left' turns left, and 'right' turns right
  val left_idx = order_from_padding(spec, left)
  val right_idx = order_from_padding(spec, right)
  and {
    left_idx != None,
    right_idx != None,
    left_idx.unwrap() + 1 == right_idx.unwrap()
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// is_left_neighbor returns true if `right` is the next possible path
/// right of `left`
///
/// Find the common suffix from the Left.Path and Right.Path and remove it.
/// We have LPath and RPath now, which must be neighbors.
/// Validate that LPath[len-1] is the left neighbor of RPath[len-1].
/// For step in LPath[0..len-1], validate step is right-most node.
/// For step in RPath[0..len-1], validate step is left-most node.
```
-->
## Is Left Neighbor

`is_left_neighbor` function returns true if `right` is the next possible path right of `left`. This function emulates [`ensure_left_neighbor`](https://github.com/cosmos/ics23/blob/a31bd4d9ca77beca7218299727db5ad59e65f5b8/rust/src/verify.rs#L247) Rust function.

```bluespec "definitions" +=
def is_left_neighbor(spec: InnerSpec, lpath: List[InnerOp], rpath: List[InnerOp]): bool = {
  // count common tail (from end, near root)
  // cut the left and right paths
  lpath.indices().exists(li =>
    rpath.indices().exists(ri => and {
      // they are equidistant from the root
      length(lpath) - li == length(rpath) - ri,
      // The distance to the root (the indices are 0-based).
      // dist == 0 holds for the root.
      val dist = length(lpath) - 1 - li
      // the prefixes and suffixes match just above the cut points
      1.to(dist).forall(k =>
        val lnode = lpath[li + k]
        val rnode = rpath[ri + k]
        and {
          lnode.prefix == rnode.prefix,
          lnode.suffix == rnode.suffix
        }
      ),
```

This part emulates the first part of `ensure_left_neighbor` Rust function:

```rust
  let mut mut_left = left.to_vec();
  let mut mut_right = right.to_vec();

  let mut top_left = mut_left.pop().unwrap();
  let mut top_right = mut_right.pop().unwrap();

  while top_left.prefix == top_right.prefix && top_left.suffix == top_right.suffix {
      top_left = mut_left.pop().unwrap();
      top_right = mut_right.pop().unwrap();
  }
```
<!--- 
```bluespec "definitions" +=
      // Now topleft and topright are the first divergent nodes
      // make sure they are left and right of each other.
      // Actually, lpath[li] and rpath[ri] are an abstraction
      // of the same tree node:
      //  the left one stores the hash of the right one, whereas
      //  the right one stores the hash of the left one.
      <<<extensionLeftNeighbor>>>
```
--->
Now topleft and topright are the first divergent nodes, and algorithm makes sure they are left and right of each other. Actually, `lpath[li]` and `rpath[ri]` are an abstraction of the same tree node:

- the left one stores the hash of the right one, whereas
- the right one stores the hash of the left one.

Since we have wrapped all checks in one _big_ `and`, we can just call `isLeftStep` which will emualte the following Rust code:

```rust
  if !is_left_step(spec, &top_left, &top_right)? {
      bail!("Not left neighbor at first divergent step");
  }
```

```bluespec "extensionLeftNeighbor" +=
is_left_step(spec, lpath[li], rpath[ri]),
```

<!--- 
```bluespec "extensionLeftNeighbor" +=
// left and right are remaining children below the split,
// ensure left child is the rightmost path, and visa versa
```
--->
Left and right are remaining children below the split, and algorithm ensures left child is the rightmost path, and visa versa. This emulates the following Rust code:

```rust
ensure_right_most(spec, &mut_left)?;
ensure_left_most(spec, &mut_right)?;
```

```bluespec "extensionLeftNeighbor" +=
is_right_most(spec, lpath.slice(0, li)),
is_left_most(spec, rpath.slice(0, ri)),
```
<!--- 
```bluespec "definitions" +=
    })
  )
}
```
--->
Here is the full Rust implementation of `ensure_left_neighbor` Rust function.

```rust
fn ensure_left_neighbor(
    spec: &ics23::InnerSpec,
    left: &[ics23::InnerOp],
    right: &[ics23::InnerOp],
) -> Result<()> {
  let mut mut_left = left.to_vec();
  let mut mut_right = right.to_vec();

  let mut top_left = mut_left.pop().unwrap();
  let mut top_right = mut_right.pop().unwrap();

  while top_left.prefix == top_right.prefix && top_left.suffix == top_right.suffix {
      top_left = mut_left.pop().unwrap();
      top_right = mut_right.pop().unwrap();
  }

  if !is_left_step(spec, &top_left, &top_right)? {
      bail!("Not left neighbor at first divergent step");
  }

  ensure_right_most(spec, &mut_left)?;
  ensure_left_most(spec, &mut_right)?;
  Ok(())
}
```
