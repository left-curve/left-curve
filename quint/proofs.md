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
- We defined `InnerOp` differently than Rust implementation as well. The implementation additionally stores the hashing function, and since we fix the spec to Grug JellyFish Merkle Tree, we do not have to carry it around.
```bluespec "definitions" +=
type InnerOp = {
  prefix: Term_t,
  suffix: Term_t
}

```
- We defined `ExistenceProof` as follows. `ExistenceProof.leaf` is never used, but since it is defined in [`cosmos.ics23.v1.rs`](https://github.com/cosmos/ics23/blob/master/rust/src/cosmos.ics23.v1.rs), we decided to keep it and mimic the proto message fully. 
```bluespec "definitions" += 
type ExistenceProof = {
  key: BitArray,
  value: BitArray,
  leaf: LeafOp,
  path: List[InnerOp],
}

```
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

## ICS23 proving existence

## Get left neighbor

## Get right neighbor

## Generating commitment proof
