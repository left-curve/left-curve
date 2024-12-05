# Grug JellyFish Merkle Tree Quint specification

This is a Quint model of a Grug Jellyfish Merkle Tree implemented in Rust. The primary objective of this specification is to formalize the design of the Grug Jellyfish Merkle Tree. The design is described using Quint along with the correctness conditions in the form of invariants.

Currently, there is no formal link between the specifications and the Rust implementation. However, the Quint specs are sufficiently developed to enable the generation of traces and the creation of interesting test data in future stages of the project. This specification contains:

- Functionalities:
  - Tree manipulation. We implemented tree manipulation in two ways:
    - Rust-like implementation
      We named that algorithm [`apply_fancy`](./quint/apply_fancy.qnt). We documented its correlation to Rust in [tree_manipulation.md](./quint/tree_manipulation.md) document.
    - Simple implementation
      We implemented another algorithm for tree manipulation. This one is much simpler, therefore named [`apply_simple`](./quint/apply_simple.qnt). This one is useful for efficient test data generation.
      We tested equivalence of [`apply_fancy`](./quint/apply_fancy.qnt) and [`apply_simple`](./quint/apply_simple.qnt) in [`tree_test.qnt`](./quint/test/tree_test.qnt). Equivalence tests are specified in [`run simpleVsFancyTest`](./quint/test/tree_test.qnt#L13-L22) and [`run simpleVsFancyMultipleRepsTest`](./quint/test/tree_test.qnt#L24-L39).
  - Data types related to proofs in [`proof_types.md`](./quint/proof_types.md)
  - Generating ICS23 proof in [`proofs.md`](./quint/proofs.md)
  - ICS23 proof verification in [`grug_ics23.md`](./quint/grug_ics23.md)

- Invariants are described in [invariants.md](./quint/invariants.md) document
<!--- TODO: fix this --->
- Experimental evaluation is here (simulation log)
<!--- TODO: fix this --->
- Preliminary verification results are here
