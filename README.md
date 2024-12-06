# Grug JellyFish Merkle Tree Quint specification

This is a Quint model of a Grug Jellyfish Merkle Tree implemented in Rust. The primary objective of this specification is to formalize the design of the Grug Jellyfish Merkle Tree. The design is described using Quint along with the correctness conditions in the form of invariants and tests.

Currently, there is no formal link between the specifications and the Rust implementation. However, the Quint specs are sufficiently developed to enable the generation of traces and the creation of interesting test data to validate the Rust implementation against these tests in future stages of the project. This specification contains:

- Functionalities:
  - Tree manipulation. We implemented tree manipulation in two ways:
    - Rust-like implementation
      We named that algorithm [`apply_fancy`](./quint/apply_fancy.qnt). We documented its correlation to Rust in [tree_manipulation.md](./quint/tree_manipulation.md).
    - Simple implementation
      We implemented another algorithm for tree manipulation. This one is algorithmically much simpler (compared to the Rust implementation that is optimized for performance in production), therefore named [`apply_simple`](./quint/apply_simple.qnt). This one is designed for efficient test data generation in Quint.
      We tested functional equivalence of [`apply_fancy`](./quint/apply_fancy.qnt) and [`apply_simple`](./quint/apply_simple.qnt) in [`tree_test.qnt`](./quint/test/tree_test.qnt). Equivalence tests are specified in [`run simpleVsFancyTest`](./quint/test/tree_test.qnt#L12-L19) and [`run simpleVsFancyMultipleRepsTest`](./quint/test/tree_test.qnt#L21-L35).
      That equivalence is described in [invariants.md](./quint/invariants.md#testing-functional-equivalence).
  - Data types related to proofs in [`proof_types.md`](./quint/proof_types.md)
  - Generating ICS23 proof in [`proofs.md`](./quint/proofs.md)
  - ICS23 proof verification in [`grug_ics23.md`](./quint/grug_ics23.md). We have described interesting test scenarios in [invariants.md](./quint/invariants.md#testing-proofs).

- Invariants are described in [invariants.md](./quint/invariants.md) document
- Results from experiments, including simulation, testing and model checking, are in [simulation_and_model_checking.md](./quint/simulation_and_model_checking.md)
