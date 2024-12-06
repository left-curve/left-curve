# Simulations and Model Checking

In order to obtain confidence that the model is correct, in respect to the invariants and tests described in [invariants.md](./invariants.md), we run random simulations (on bigger scopes) and model checking (on smaller scopes). We used the **Quint simulator** for simulations, which runs a kind of Depth First Search (DFS) on the state space with a max-depth (`--max-steps`) defined by us; and the **TLC model checker** for model checking, which runs a kind of Breadth First Search (BFS) on the complete state space, which requires that we refine our model to a state-space small enough for this to run in the time we had available.

## Simulations

Simulations were the main tool we used while iterating over the model. It helped us spot several small issues as soon as they appeared, which often happen due to mistakes on writing the model and the invariants. On one of these routine runs, we found one actual issue (that was reproducible on the Rust implementation but low severity), which was [reported and fixed](https://github.com/left-curve/left-curve/pull/291).

Once the model and invariants are stable and we don't get violations from the simulator on a few minutes, we can setup longer runs, which serve to increase our confidence on the model. The first one was run once our model and invariants for tree manipulation were stable, and the other ones were done after the model and invariants for proofs and proof verification were stable.

### **Simulating tree manipulation**

At the point in time where we finished tree manipulation, we ran a **16-hour simulation** with **12 parallel instances** of the simulator, each with `max-steps=3`, and **100k samples per command**. This gave us a total of **1.2M samples**. For this simulation, we limited batches to have at most 5 operations, and used `key_hash`es of length 6.

### **Simulating proofs and proof verification**

The invariants for producing and verifying proofs are quite heavy performance-wise, as they check lots of combinations of keys to generate proofs for and trees and values to check the proof against. This means that the total number of samples we can check is much lower in relation to what we check for tree manipulation, but the number of checks per sample is very high.

#### **First simulation**

We ran a **17-hour simulation** with **12 parallel instances** of the simulator, each with `max-steps=3`, and **15k samples per command**. This gave us a total of **180k samples**. For this simulation, we limited batches to have at most 5 operations, and used `key_hash`es of length 4, and used the prunning state machine. In this experiment, we only checked the proof-related invariants (completeness, soundness and the `verifyMembershipInv`). This was run at [30a7013](https://github.com/informalsystems/left-curve-jmt/pull/58/commits/30a70137328040e865a530295477359be90cd5b4).

#### **Second simulation**

Before running the next experiment, we want to try to improve two things:
1. **Performance of the invariants**
2. **Distribution of batches**

##### **Performance of the invariants**

There were some straightforward improvements that were made, but made a small compromise in favor of enabling more verification. For many invariants, we had a quantifier over all active (unprunned) tree versions. However, invariants are checked of every single state, which means that, for three steps, we had something like this:
1. Check invariant for state 0, which has an empty tree
2. Take a step and check invariant for state 1, which has one tree version
3. Take a step and check invariant for state 2, which has two different tree versions
4. Take a step and check invariant for state 3, which has three different tree versions

A tree version in this context is a projection of the tree at a given version (taking all nodes with the greatest version less or equal than that version).

This means that the tree at version 1 was checked three times at (2), (3) and (4); tree at version 2 was checked twice at (3) and (4); and tree at version 3 was checked once at (4). However, we can assume that this tree projection doesn't change on future steps, and remove this extra redundant checks. This way, instead of quantifying over all active versions on the invariants, we only check things for the latest versions, assuming that the trees from previous versions didn't change and were checked previously at their own steps.

We went with this assumption and remove the quantification over active versions from invariants that had it.

##### **Distribution of batches**

The strategy used to generate non-deterministic batches was much more likely to generate medium-sized batches that batches with no operation, a single operation or all possible operations. At this point, we had use two different batch generation strategies:
1. Produce a powerset over the set of operations and pick one.
  - In this powerset, there are much more big sets then small sets, which means that we are more likely to pick a big set.
2. For each `key_hash`, non-deterministically pick if we want to include it in the batch or not.
  - This has a central tendency issue, where we are more likely to pick a batch with half of the operations than with no operations or all operations.

What we believed would be ideal is that chances of batches of different sizes are all the same. So we changed this to first non-deterministically pick the size of the batch, and then non-deterministically pick the operations to include in the batch. This way, we have a uniform distribution of batch sizes.

We also removed the limit of 5 operations per batch, and reduced the set from which value hashes are picked from to have only two potential values, increasing the chances of collision (having an insert operation for a key that was inserted with the same value before).

##### **Simulation itself**

We ran an **8-hour simulation** with **12 parallel instances** of the simulator, each with `max-steps=3`, and **20k samples per command**. This gave us a total of **240k samples**. We used `key_hash`es of length 4. In this experiment, we checked all invariants, including the proof-related ones, except for prunning, as we used the regular state machine for performance reasons. This was run at [be6b33b](https://github.com/informalsystems/left-curve-jmt/commit/be6b33ba547901ab7e5bb4863dd54b03d4baf0ac).

### **Testing**

Another way to use the Quint random simulator is to run tests, which work like property-based testing. The tests can have non-deterministic values, so we can run many samples to get more confidence that the property stated by the tests holds. We focus the long-running testing on the more interesting ones:

#### **Simple apply vs Fancy apply**

We ran an **6-hour** test with **8 parallel instances** of the test simulator, and **10k samples per command**. This gave us a total of **80k samples** for each test. It was run for the [tree_test.qnt](./test/tree_test.qnt) file which includes two tests:
```
ok simpleVsFancyTest passed 10000 test(s)
ok simpleVsFancyMultipleRepsTest passed 10000 test(s)
```

#### **Proof verification across different trees**

We ran a **1-hour** test with **8 parallel instances** of the test simulator, and **500 samples per command**. This gave us a total of **4k samples** for each test. It was run for the [proofs_test.qnt](./test/proofs_test.qnt) file which includes the following:

```
ok twoDifferentTreesTest passed 500 test(s)
ok twoDifferentTreesByOnlyValuesTest passed 500 test(s)
ok twoDifferentTreesByOnlyOneValueTest passed 500 test(s)
ok twoDifferentTreesSameByOnlyOneKVTest passed 500 test(s)
ok verificationOnPrunnedTreeTest passed 500 test(s)
ok leafNotExistsThenExistsTest passed 500 test(s)
1) leafExistsThenNotExistsTest failed after 392 test(s)
```

- TODO: fix failing test and update

## **Model Checking**

We were able to run the TLC model checker for two different setups. We were not able to run Apalache, as it quickly ran out of memory before the model checking started (to be investigated). In order to run TLC, we transpile Quint to TLA+ and then run the model checker on the TLA+ model.

### **Generating the TLA+ model**

There are still some integration issues between Quint and Apalache, which is used to generate TLA+ out of Quint. We had to circunvent this, introducing non-dirsruptive changes:
- Some new Quint builtins (`getOnlyElement` and `allListsUpTo`) are not supported for translation yet, so we replaced those with a non-builtin version. Same for `foldr` which we adapted to use `foldl` which is supported.
- There were many issues translating polymorphic Quint operators into Apalache's representation, so we did some adaptations to avoid some intances of polymorphism, mostly regarding polymorphic usage of the `None` constructor inside the same operator.
- Many issues could only be fixed on Apalache's side, which we did. See the [PR](https://github.com/apalache-mc/apalache/pull/3041). We used a version of Apalache with this fixes in order to generate the TLA+ model.

The generated TLA+ model is available on the model-checking branch at [apply_state_machine.tla](https://github.com/informalsystems/left-curve-jmt/blob/gabriela/model-checking/quint/apply_state_machine.tla)

### **Initial states for model checking**

With the goal of optimizing the state space as much as possible for model checking, we define a special case for the initial state. Instead of always starting with an empty tree and applying any operation, we consider a symmetry property of trees and operations, where any scenario arising from applying two sets of operations of value hashes `[1]` or `[2]` on top of an empty tree should be reproducible by applying first a set of operations with only value hash `[1]` and then the second set with `[1]` or `[2]`. This reduces the number of operation combinations to consider, while maintaining the same coverage.

Therefore, we change the `init` definition to start with the result of applying a non-deterministic set of operations with value hash `[1]` on top of an empty tree.

This also means that performing a single step in this state machine is similar to performing two steps in the original state machine that always started with an empty tree, as we are now also applying one batch of operations on the initial state.

### **Setup A**

- The key hash length is 3
- The state machine performs a single step

```
Model checking completed. No error has been found.
  Estimates of the probability that TLC did not check all reachable states
  because two distinct states had the same fingerprint:
  calculated (optimistic):  val = 0.0
16777472 states generated, 16777472 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 2.
The average outdegree of the complete state graph is 0 (minimum is 0, the maximum 31 and the 95th percentile is 0).
Finished in 01h 55min at (2024-12-05 22:58:14)
```

- Running this with 2 steps instead of 1 would increase the state space to **1 099 528 405 248 states**
- Running this with key hash length of 4 instead of 3 would increase the state space to **281 474 976 710 656 states**

An inductive interpretation of this result is:
- **Base case:** All possible trees with `value_hash` being `[1]` and version being `1`.
- **Induction step:** All possible operations with `value_hash` being `[1]` (same as a potentially existing leaf) or `[2]` (different from a potentially existing leaf), from version `1` to version `2`.

This check is a valid inductive proof for the real tree manipulation algorithm if and only if the following assumptions hold:
- Any violation for key hashes of length 256 can be reproduced with key hashes of length 3
- Any violation for value hashes of length 256 can be reproduced using only two value hashes (specifically `[1]` and [`2`])
- Any violation that happens in multiple steps/versions can be reproduced in a single step/version change (specifically from version `1` to version `2`). See Setup B for more coverage on this.
- Our model is equivalent to the algorithm. Model-based testing can help obtaining confidence on this.

### **Setup B**

- The key hash length is 2
- The state machine performs 2 steps

```
Model checking completed. No error has been found.
  Estimates of the probability that TLC did not check all reachable states
  because two distinct states had the same fingerprint:
  calculated (optimistic):  val = 0.0
1052688 states generated, 1052688 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.
The average outdegree of the complete state graph is 0 (minimum is 0, the maximum 31 and the 95th percentile is 0).
Finished in 02min 46s at (2024-12-05 20:53:41)
```

- Running this with 3 steps instead of 2 would increase the state space to **269 488 144 states**
  - We estimate that this will take about a week to run and use ~200GB of disk space
- Running this with 4 steps instead of 3 would increase the state space to **68 988 964 880 states**

An inductive interpretation of this result is:
- **Base case:** All possible trees that can be generated using all possible combinations of operations with `value_hash` being `[1]` or `[2]`, and versions being 1 and 2.
- **Induction step:** All possible operations with `value_hash` being `[1]` (same as a potentially existing leaf) or `[2]` (different from a potentially existing leaf), from version `2` to version `3`.

This check is a valid inductive proof for the real tree manipulation algorithm if and only if the following assumptions hold:
- Any violation for key hashes of length 256 can be reproduced with key hashes of length 2
- Any violation for value hashes of length 256 can be reproduced using only two value hashes (specifically `[1]` and [`2`])
- Any violation that happens in multiple steps/versions can be reproduced with 2 steps/versions
- Our model is equivalent to the algorithm. Model-based testing can help obtaining confidence on this.
