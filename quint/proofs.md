# Tree manipulation

This document describes how ics23 proof generation was modelled in Quint, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible. However, since Quint does not support hashing, there were some challenges.

Most of the correspondance is shown by comparing the Rust code with Quint code short snippets at a time. The most complicated correspondance is on early returns, which do not exist in Quint. We are explaining this in detail on [].

This document covers:

- [`ics23_prove_existence`](#ics23-proving-existence)
- [`leftNeighbor`](#get-left-neighbor)
- [`rightNeighbor`](#get-right-neighbor)
- [`ics23_prove`](#generating-commitment-proof)

> [!TIP]
> This markdown file contains some metadata and comments that enable it to be tangled to a full Quint file (using [lmt](https://github.com/driusan/lmt)). The Quint file can be found at [proofs.qnt](./proofs.qnt).

<!-- Boilerplate: tangled from comment to avoid markdown rendering
```bluespec qroofs.qnt
// -*- mode: Bluespec; -*-

module proofs {
  
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"
  
  <<<definitions>>>
  
}
```
-->
## Types

Types used are similar to the original Rust implementation. There are some minor differences, but those will be addressed in detail here. We based our types on [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs) file.

- We defined `LeafOp` differently than Rust implementation. The implementation additionally stores hashing and length functions: hash, prehashKey, prehashValue, len. Since we fix the spec to Grug JellyFish Merkle Tree, we do not have to carry them around.

```bluespec "definitions" +=
type LeafOp = {
  prefix: Term_t
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
- We defined `InnerOp` differently than Rust implementation as well. The implementation additionally stores the hashing function, and since we fix the spec to Grug JellyFish Merkle Tree, we do not have to carry it around.

```bluespec "definitions" +=
type InnerOp = {
  prefix: Term_t,
  suffix: Term_t
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
- We defined `ExistenceProof` as follows. `ExistenceProof.leaf` is never used, but since it is defined in [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs), we decided to keep it and mimic the proto message fully.

```bluespec "definitions" +=
type ExistenceProof = {
  key: BitArray,
  value: BitArray,
  leaf: LeafOp,
  path: List[InnerOp],
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
- We defined `NonExistenceProof` as follows.

```bluespec "definitions" +=
type NonExistenceProof = {
  key: BitArray,
  left: Option[ExistenceProof],
  right: Option[ExistenceProof],
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->
- We defined `CommitmentProof` as follows.

```bluespec "definitions" +=
type CommitmentProof =
  | Exist(ExistenceProof) 
  | NonExist(NonExistenceProof)
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

/// Returns optional list of InnerOps as a path to the leaf with particular key_hash
```
-->

## ICS23 proving existence

Like mentioned in [tree_manipulation.md](./tree_manipulation.md), Quint does not support early breaks, which meant that we had to improvise.
First, the signature:

```rust
/// Traverse the tree, find the leaf node containing the key hash, and
/// return the ICS-23 path (the list of `InnerOp`'s) that can prove this
/// key's existence.
///
/// ## Panics
///
/// Panics if the key is not found. The caller must ensure the key exists
/// before calling. This is typically done by querying the state storage
/// first.
pub fn ics23_prove_existence(
  &self,
  storage: &dyn Storage,
  version: u64,
  key_hash: Hash256,
) -> StdResult<Vec<InnerOp>> {
```

```bluespec "definitions" +=
pure def ics23_prove_existence(t: Tree, version: Version, key_hash: BitArray) 
  : Option[List[InnerOp]] =
```

As clearly visible, the function signature is differnt. In our specification, we are returning `Option[List[InnerOp]]`. Returning `None` is reserved for any error that has occured when traversing the tree. In Rust implementation, looping is done until leaf is reached. However, since Quint does not support early returns, we had to have a finite set of key prefixes that we can fold over.

```bluespec "definitions" +=
  val prefixes_list = 0.to(key_hash.length()).map( i => key_hash.slice(0,i)).toList(listCompare)
```

We set our `iterator` to a record that stores `path` - empty list, index of `key_prefix` in a `prefixes_list` - `0`, boolean value which indicates wether leaf has been found - `false` and a `child_version` which helps us find the correct version of internal node's child - `version` for which `ics23_prove_existence` is called.

```bluespec "definitions" +=
    val r = prefixes_list.foldl({ path: List(), i: 0, found: false, child_version: version}, (iterator, key_prefix) => 
```

`iterator.found` is used in our early return workaround. If the algorithm has found the leaf with adequate `key_hash` or if it cannot find a leaf with corresponding `key_prefix` and `iterator.child_version`, iterator won't be changed until fold has finished.

```bluespec "definitions" +=
    if (iterator.found or not(t.nodes.keys().contains({key_hash: key_prefix, version: iterator.child_version}))) 
        iterator 
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=
    else
```
-->

In first fold iteration `key_prefix` will take ROOT_BITS value and root will be fetched from the state.

```bluespec "definitions" +=
        val node = t.nodes.get({key_hash: key_prefix, version: iterator.child_version})
```

This corresponds to the following Rust parts of code: 

```rust
let mut bits = ROOT_BITS;
...
let mut node = self.nodes.load(storage, (version, &bits))?;
```

After node has been fetched from the tree, we perform match operation, because node can either be `Leaf` or `Internal`. If the algorithm has reached the leaf, iterator will take value: `{ ...iterator, i: iterator.i + 1, found: l.key_hash == key_hash }`, `...iterator` being the spread operator which will take values of old iterator and apply it to new one. This means that if `key_hashes` are the same, we have found the leaf we are looking for and we should early break, therefore we are putting setting adequate value for `iterator.found`.  However if the algorithm has reached an internal node, that means that there is more things to do to reach the leaf. Here is the `match` statement in Rust implementation, that we emulated in Quint. As previously mentioned, we will not have record of `hash` in our `InnerOp` type.
```rust
match node {
  Node::Leaf(leaf) => {
      assert_eq!(leaf.key_hash, key_hash, "target key hash not found");
      break;
  },
  Node::Internal(InternalNode {
      left_child,
      right_child,
  }) => match (iter.next(), left_child, right_child) {
      (Some(0), Some(child), sibling) => {
          bits.push(0);
          node = self.nodes.load(storage, (child.version, &bits))?;
          path.push(InnerOp {
              // Not sure why we have to include the `HashOp` here
              // when it's already in the `ProofSpec`.
              hash: ICS23_PROOF_SPEC.inner_spec.as_ref().unwrap().hash,
              prefix: INTERNAL_NODE_HASH_PREFIX.to_vec(),
              suffix: sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).to_vec(),
          });
      },
      (Some(1), sibling, Some(child)) => {
          bits.push(1);
          node = self.nodes.load(storage, (child.version, &bits))?;
          path.push(InnerOp {
              hash: ICS23_PROOF_SPEC.inner_spec.as_ref().unwrap().hash,
              prefix: [
                  INTERNAL_NODE_HASH_PREFIX,
                  sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).as_ref(),
              ]
              .concat(),
              suffix: vec![],
          })
      },
      _ => unreachable!("target key hash not found"),
  },
}
```

Since Quint's pattern matching is not as strong as Rust's we had to figure a way around it. First we will append `0` to the `key_prefix` and then try to see if in the `prefixes_list`, the next element will have the same `key_prefix`.

```bluespec "definitions" +=
          match node {
            | Leaf(l) => { ...iterator, i: iterator.i + 1, 
                                      found: l.key_hash == key_hash }
            | Internal(internal) => 
              val next_bit_0 = key_prefix.append(0)
              val child_version = if(prefixes_list[iterator.i + 1] == next_bit_0) 
                internal.left_child.unwrap().version
                else 
                internal.right_child.unwrap().version
```

Then, we will take its version and declare it `child_version`. After doing so, we check wether we should end up in the `(Some(0), Some(child), sibling)` or `(Some(1), sibling, Some(child))` `match` branch.

- If we are to end up in the `(Some(0), Some(child), sibling)` branch, that means that we will create `innerOp` variable with `prefix` being `InternalNodeHashPrefix` and `suffix` being either hash of the right child or zeroed out hash (Hash256_ZERO).

```bluespec "definitions" +=
              val innerOp = 
                if(prefixes_list[iterator.i + 1] == next_bit_0) 
                  { prefix: InternalNodeHashPrefix, 
                    suffix: match internal.right_child {
                              | None => Hash256_ZERO
                              | Some(c) => c.hash} }
```

This part closely resembles this part of Rust implementation:

```rust
  path.push(InnerOp {
      // Not sure why we have to include the `HashOp` here
      // when it's already in the `ProofSpec`.
      hash: ICS23_PROOF_SPEC.inner_spec.as_ref().unwrap().hash,
      prefix: INTERNAL_NODE_HASH_PREFIX.to_vec(),
      suffix: sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).to_vec(),
  });
```

- If we are to end up in the `(Some(1), sibling, Some(child))` branch, that means that we will create `innerOp` variable with `prefix` being concatanated values of InternalNodeHashPrefix and either hash of the left child or zeroed out hash (Hash256_ZERO). In this case, `suffix` will be an empty Map(), which corresponds to an empty vector in Rust implementation.

```bluespec "definitions" +=
                else 
                  { prefix: InternalNodeHashPrefix
                              .termConcat(match internal.left_child {
                                            | None => Hash256_ZERO
                                            | Some(c) => c.hash}), 
                    suffix: Map() }
```

This part closely resembles this part of Rust implementation:

```rust
  path.push(InnerOp {
    hash: ICS23_PROOF_SPEC.inner_spec.as_ref().unwrap().hash,
    prefix: [
        INTERNAL_NODE_HASH_PREFIX,
        sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).as_ref(),
    ]
    .concat(),
    suffix: vec![],
  })
```

After creating new innter op, we update iterator with new values, such as new index of `key_prefix`, `child_version` that we have created before creating `innerOp` and new entry in the `path` list.

```bluespec "definitions" +=
              { ...iterator, path: iterator.path.append(innerOp),  
                i: iterator.i + 1, child_version: child_version }
          }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=
    )
```
-->
After completing the fold, we are supposed to reverse the path:

```rust
  // The path goes from bottom up, so needs to be reversed.
  path.reverse();
```

Our specification does the same, in the event that we have found the leaf we were looking for. Since the Rust implementation will panic when there is no leaf with the wanted `key_hash`, in order to handle panics, we decided to return `None` in the event we did not find the leaf we were looking for, and reverse the path if we did.
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=
```
-->
```bluespec "definitions" +=
    if (r.found)
      Some(r.path.foldr(List(), (path_element, reversed_path ) => 
                                          reversed_path.append(path_element)))    
    else
      None
```

## Get left neighbor

## Get right neighbor

## Generating commitment proof
