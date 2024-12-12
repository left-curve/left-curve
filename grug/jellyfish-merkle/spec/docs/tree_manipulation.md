# Tree manipulation

_This document was prepared by the [Informal Systems security team](https://informal.systems/security)_

This document describes how tree manipulation was modelled in Quint, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible. However, Rust and Quint are very different, so there are some challenges:

- The Rust implementation uses recursion, which Quint doesn't support (due to Apalache)
- Rust has mutable variables, while Quint is functional and does not
- Pattern matching in Rust is more powerful than in Quint
- Rust has early returns while Quint is functional and does not
- Rust has statement if while Quint is functional and does not (every `if` needs an `else`).

Most of the correspondance is shown by comparing the Rust code with Quint code short snippets at a time. The most complicated correspondance is on the recursion emulation, which we explain in more detail on [Recursion emulation for `apply_at`](#recursion-emulation-for-apply_at) and [Recursion emulation for `create_subtree`](#recursion-emulation-for-create_subtree).

This document covers the correspondance of all the `apply_*` functions and their main helper functions, including:

- [`apply`](#top-level-apply)
- [`apply_at`](#apply-at)
- [`apply_at_internal`](#apply-at-internal)
- [`apply_at_child`](#apply-at-child)
- [`apply_at_leaf`](#apply-at-leaf)
- [`partition_batch`](#partition-batch)
- [`partition_leaf`](#partition-leaf)
- [`prepare_batch_for_subtree`](#prepare-batch-for-subtree)
- [`create_subtree`](#create-subtree)

> [!TIP]
> This markdown file contains some metadata and comments that enable it to be tangled to a full Quint file (using [lmt](https://github.com/driusan/lmt)). The Quint file can be found at [apply_fancy.qnt](../quint/apply_fancy.qnt).

<!-- Boilerplate: tangled from comment to avoid markdown rendering
```bluespec quint/apply_fancy.qnt
// -*- mode: Bluespec; -*-

// Grug's algorithm to apply batches of operations to Jellyfish Merkle Trees
//
// Josef Widder, Informal Systems, 2024
// Aleksandar Ignjatijevic, Informal Systems, 2024
// Gabriela Moreira, Informal Systems, 2024

module apply_fancy {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import utils.* from "./utils"

  import basicSpells.* from "./spells/basicSpells"
  import commonSpells.* from "./spells/commonSpells"
  import rareSpells.* from "./spells/rareSpells"

  <<<definitions>>>
}
```
-->

## Types

Types have a 1:1 mapping between Rust and Quint that is pretty trivial, so won't be covered here. See [tree.qnt](../quint/tree.qnt) and [node.qnt](../quint/node.qnt) for types. The only difference is that we chose to use records instead of tuples for most things, as accessing records is more readable than accessing tuples, specially since Quint doesn't support destructuring tuples at this time.

In order to replicate Rust's mutations over the tree in Quint, we need to also return the modified values. So, we define a type for the return of apply operations:

```bluespec "definitions" +=
/// The return type of apply_* functions, as some of them modify the tree
type ApplyResult = { outcome: Outcome, orphans_to_add: Set[OrphanId], nodes_to_add: Set[(NodeId, Node)] }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

## Recursion emulation for `apply_at`

The Rust functions `apply_at`, `apply_at_internal` and `apply_at_child` are mutually recursive. Since there is no recursion in Quint, we need to emulate it. What happens in the actual recursion is that longer bit arrays will get computed first, and then smaller bit array computations might depend on the result of longer bit array computations. Therefore, to emulate it, we start from the longest bit arrays, which shouldn't depend on anything else, and save the result in a `memo` value that will be given to what we process next. Then, when a computation calls `apply_at`, we read a result from the `memo` value instead - and the result should already be there, since are computing dependencies first.

The type of the `memo` value is defined by `ApplyAtMemo`, and has a similar signature to the `apply_at` Rust function. `sorted_nodes` defines the order in which memo values will be pre-computed, and `pre_compute_apply_at` does iteration over versions and nodes, pre-computing `apply_at` for each of them. Following, let's look at a concrete example and understand this emulation in more detail.

```bluespec "definitions" +=
/// Result of apply_at calls, for emulating recursion
type ApplyAtMemo = (Version, Version, BitArray, Set[OperationOnKey]) -> ApplyResult

pure def sorted_nodes(tree: Tree): List[NodeId] = {
  tree.nodes.keys().toList((a, b) => intCompare(a.key_hash.length(), b.key_hash.length()))
}

/// Pre computation of apply_at used to emulate recursion
pure def pre_compute_apply_at(tree: Tree, new_version: Version, batch: Set[OperationOnKey]): ApplyAtMemo = {
  range(0, new_version).foldl(Map(), (memo, old_version) => {
    sorted_nodes(tree).foldr(memo, (node_id, memo) => {
      pure val bits = node_id.key_hash
      pure val batch_here = batch.filter(o => bits.prefix_of(o.key_hash))

      pure val memo_key = (new_version, old_version, bits, batch_here)
      pure val memo_value = tree.apply_at(memo, new_version, old_version, bits, batch_here)
      memo.put(memo_key, memo_value)
    })
  })
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Concrete example

Consider the following tree:

```plain
       root
     ┌──┴──┐
    (0)    1
  ┌──┴──┐
 00    (01)
     ┌──┴──┐
    010   011
```

With `010` being a prefix for `0100` and `011` being a prefix `0110`. Consider the following batch:

```plain
Delete 0100
Insert 0110 <new value>
```

The call stack for the Rust implementation looks something like this:

```rust
apply_at(ROOT_BITS, [ Delete 0100, Insert 0110 ])
↳ apply_at_internal(ROOT_BITS, [ Delete 0100, Insert 0110 ])
   ↳ apply_at_child(0, [ Delete 0100, Insert 0110 ])
      ↳ apply_at(0, [ Delete 0100, Insert 0110 ])
         ↳ apply_at_internal(0, [ Delete 0100, Insert 0110 ])
            ↳ apply_child(00, [ ])
            ↳ apply_child(01, [ Delete 0100, Insert 0110 ])
               ↳ apply_at(01, [ Delete 0100, Insert 0110 ])
                  ↳ apply_at_internal(01, [ Delete 0100, Insert 0110 ])
                     ↳ apply_at_child(010, [ Delete 0100 ])
                        ↳ (...) Mark as orphan [mutation]
                        ↳ (...) Outcome: Deleted
                     ↳ apply_at_child(011, [ Insert 0110 ])
                        ↳ (...) Mark as orphan [mutation]
                        ↳ (...) Outcome: Updated
      ↳ Left was Unchanged, Right was Updated => save [mutation]
   ↳ apply_at_child(1, [ ])
   ↳ Left was Updated, Right was Unchanged => save [mutation]
```

To emulate recursion in Quint, we want to process the rightmost parts of the above call tree first. Considering only the `apply_at` calls, that is:

```rust
1. apply_at(010, [ Delete 0100 ])
2. apply_at(011, [ Insert 0110 ])
3. apply_at(00, [ ])
4. apply_at(01, [ Delete 0100, Insert 0110 ])
5. apply_at(0, [ Delete 0100, Insert 0110 ])
6. apply_at(1, [ ])
7. apply_at(ROOT_BITS, [ Delete 0100, Insert 0110 ])
```

We want to compute the results of these calls in this order, and then use the results of the longer bit arrays to compute the shorter ones. We will use the `pre_compute_apply_at` function to compute these results.

### Memoization

The example above only talks about `bits` and `batch`, but recursive calls of `apply_at` also include the arguments: `self`, `storage`, `new_version` and `old_version`. `self` and `storage` are only used to load the tree, which is always done for versions older than the current one being built, so it should be the same for every single call - therefore, we don't make this as part of our key in the `memo` value, as it would introduce significant complexity on figuring out how the tree looked like at that call, and we are not going to read it anyway.

> [!IMPORTANT]
> Not tracking the tree state might be the biggest difference between model and implementation. It does seem like a fair assumption since we always read the tree from older versions, but it might be worth to find a stronger argument for this.

The other fields (`new_version`, `old_version`, `bits` and `batch`) are relevant, and we need to include all of them in the `memo` key so it doesn't end up returning a computation for a full batch while the implementation, at that point, would only have a partial batch. It's easy to predict what batches would be given as arguments to what bits, and we also how the `new_version` from the original call. `bits` can be any of the existing key hashes of the existing tree, and we want to iterate from the longest ones to the shortest ones. We couldn't find a good way to predict `old_version`, so we actually compute `memo` values for all possible versions from `0` to `new_version`, and then we know that it will have a value for whatever possible `old_version` it's called with.

> [!NOTE]
> There might be a way to predict `old_version` for each call and avoid unnecessary computations. We should look into this in the future. This does _not_ affect correctness, only performance.

## Apply operators

### Top level apply

First, the signature:

```rust
pub fn apply(
    &self,
    storage: &mut dyn Storage,
    old_version: u64,
    new_version: u64,
    batch: Vec<(Hash256, Op<Hash256>)>,
) -> StdResult<Option<Hash256>> {
```

```bluespec "definitions" +=
pure def apply(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]): Tree = {
```

Some things to note:

1. Quint takes and returns a tree instead of self + storage, as it is a functional language and doesn't have mutability.
2. The batch is a set in Quint instead of a vector, but that is ok because batches in rust originate from maps and are guaranteed to be unique and idependent of ordering.
3. While Rust returns an outcome, Quint only returns a tree. There are no verification goals for the outcome, so we ignore it. We return a tree because of (1).
4. In general, we don't need to handle errors in Quint as the only errors in Rust are related to storage failures, which we don't model in Quint.

Then we have a debug assertion in Rust, which we ignore in Quint. This is just input validation, so we only need to ensure we call this with the correct parameters on tests and the state machine.

```rust
  // The caller must make sure that versions are strictly incremental.
  // We assert this in debug mode must skip in release to save some time...
  debug_assert!(
      new_version == 0 || new_version > old_version,
      "version is not incremental"
  );
```

Then, we might need to mark the node as orphaned.

```rust
  // If an old root node exists (i.e. tree isn't empty at the old version),
  // mark it as orphaned.
  if self.nodes.has(storage, (old_version, &ROOT_BITS)) {
      self.mark_node_as_orphaned(storage, new_version, old_version, ROOT_BITS)?;
  }
```

While Rust has a statement if with a mutation inside it. In Quint, we assign the new value to `tree_1`, and add an `else` branch that returns the unmodified tree. We use `tree_1` everywhere after this.

```bluespec "definitions" +=
  pure val tree_1 =
    if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
      tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
    } else {
      tree
    }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Lastly, we make the recursive call

```rust
    // Recursively apply the ops, starting at the old root.
    match self.apply_at(storage, new_version, old_version, ROOT_BITS, batch)? {
        // If the new tree is non-empty (i.e. it has a root node), save this
        // new root node and return its hash.
        Outcome::Updated(new_root_node) | Outcome::Unchanged(Some(new_root_node)) => {
            self.save_node(storage, new_version, ROOT_BITS, &new_root_node)?;
            Ok(Some(new_root_node.hash()))
        },
        // The new tree is empty. do nothing and just return `None`.
        Outcome::Deleted | Outcome::Unchanged(None) => Ok(None),
    }
}
```

As explained, we call `pre_compute_apply_at` to prepare for the recursion from calling `apply_at`.

- `memo`, the result from the pre-computation, is given to `apply_at` so it uses it instead of doing recursion.
- The result obtained from `apply_at` includes mutations to be made to the tree (`nodes_to_add` and `orphans_to_add`). We produce a new tree (`new_tree`) with those changes applied.
- Then, we just need to match, in a less powerful but similar pattern matching.

```bluespec "definitions" +=
  pure val memo = pre_compute_apply_at(tree_1, new_version, batch)
  pure val apply_result = tree_1.apply_at(memo, new_version, old_version, ROOT_BITS, batch)
  pure val new_tree = {
    nodes: tree_1.nodes.add_nodes(apply_result.nodes_to_add),
    orphans: tree_1.orphans.union(apply_result.orphans_to_add)
  }

  match apply_result.outcome {
    | Updated(new_root_node) => {
        ...new_tree,
        nodes: new_tree.nodes.put({ version: new_version, key_hash: ROOT_BITS }, new_root_node)
      }
    | Unchanged(optional) => {
        match optional {
          | Some(new_root_node) => {
              ...new_tree,
              nodes: new_tree.nodes.put({ version: new_version, key_hash: ROOT_BITS }, new_root_node)
            }
          | None => new_tree
        }
      }
    | _ => new_tree
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Apply At

```rust
fn apply_at(
    &self,
    storage: &mut dyn Storage,
    new_version: u64,
    old_version: u64,
    bits: BitArray,
    batch: Vec<(Hash256, Op<Hash256>)>,
) -> StdResult<Outcome> {
    match self.nodes.may_load(storage, (old_version, &bits))? {
        Some(Node::Leaf(leaf_node)) => {
            self.apply_at_leaf(storage, new_version, bits, leaf_node, batch)
        },
        Some(Node::Internal(internal_node)) => {
            self.apply_at_internal(storage, new_version, bits, internal_node, batch)
        },
        None => {
            let (batch, op) = prepare_batch_for_subtree(batch, None);
            debug_assert!(op.is_none());
            self.create_subtree(storage, new_version, bits, batch, None)
        },
    }
}
```

Quint needs one more level of `match` and doesn't have the debug statement.

```bluespec "definitions" +=
pure def apply_at(
  tree: Tree,
  memo: ApplyAtMemo,
  new_version: Version,
  old_version: Version,
  bits: BitArray,
  batch: Set[OperationOnKey]
): ApplyResult = {
  match (tree.nodes.safeGet({ version: old_version, key_hash: bits })) {
    | Some(node) => match node {
      | Leaf(leaf_node) => tree.apply_at_leaf(new_version, bits, leaf_node, batch)
      | Internal(internal_node) => tree.apply_at_internal(memo, new_version, bits, internal_node, batch)
    }
    | None => {
      pure val batchAndOp = prepare_batch_for_subtree(batch, None)
      tree.create_subtree(new_version, bits, batchAndOp._1, None)
    }
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Apply At Internal

From Rust:

```rust
fn apply_at_internal(
    &self,
    storage: &mut dyn Storage,
    new_version: u64,
    bits: BitArray,
    mut internal_node: InternalNode,
    batch: Vec<(Hash256, Op<Hash256>)>,
) -> StdResult<Outcome> {
    // Split the batch into two, one for left child, one for right.
    let (batch_for_left, batch_for_right) = partition_batch(batch, bits);
```

Nothing outstanding in Quint:

```bluespec "definitions" +=
pure def apply_at_internal(
  tree: Tree,
  memo: ApplyAtMemo,
  new_version: Version,
  bits: BitArray,
  internal_node: InternalNode,
  batch: Set[OperationOnKey]
): ApplyResult = {
  // Split the batch into two, one for left child, one for right.
  pure val partitioned_batch = partition_batch(batch, bits)
  pure val batch_for_left = partitioned_batch._1
  pure val batch_for_right = partitioned_batch._2
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

There should also be no surprises in `apply_at_child` calls:

```rust
    // Apply the left batch at left child
    let left_bits = bits.extend_one_bit(true);
    let left_outcome = self.apply_at_child(
        storage,
        new_version,
        left_bits,
        internal_node.left_child,
        batch_for_left,
    )?;

    // Apply the right batch at right child
    let right_bits = bits.extend_one_bit(false);
    let right_outcome = self.apply_at_child(
        storage,
        new_version,
        right_bits,
        internal_node.right_child,
        batch_for_right,
    )?;
```

```bluespec "definitions" +=
  // Apply the left batch at left child
  pure val left_bits = bits.append(0)
  pure val left_result = tree.apply_at_child(memo, new_version, left_bits, internal_node.left_child, batch_for_left)
  pure val left_outcome = left_result.outcome

  // Apply the right batch at right child
  pure val right_bits = bits.append(1)
  pure val right_result = tree.apply_at_child(memo, new_version, right_bits, internal_node.right_child, batch_for_right)
  pure val right_outcome = right_result.outcome
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Next, we deal with orphans.

```rust
    // If the left child exists and have been updated or deleted, then the
    // old one needs to be marked as orphaned.
    if let (Outcome::Updated(_) | Outcome::Deleted, Some(left_child)) =
        (&left_outcome, &internal_node.left_child)
    {
        self.mark_node_as_orphaned(storage, new_version, left_child.version, left_bits)?;
    }

    // If the right child exists and have been updated or deleted, then the
    // old one needs to be marked as orphaned.
    if let (Outcome::Updated(_) | Outcome::Deleted, Some(right_child)) =
        (&right_outcome, &internal_node.right_child)
    {
        self.mark_node_as_orphaned(storage, new_version, right_child.version, right_bits)?;
    }
```

Again, no mutations on Quint, so instead we return sets of orphans to be added (which are empty on else branches).

```bluespec "definitions" +=
  // If the left child exists and have been updated or deleted, then the
  // old one needs to be marked as orphaned.
  pure val left_orphans = if (left_outcome.is_updated_or_deleted() and internal_node.left_child != None) {
    Set({
      orphaned_since_version: new_version,
      version: internal_node.left_child.unwrap().version,
      key_hash: left_bits,
    })
  } else Set()

  // If the right child exists and have been updated or deleted, then the
  // old one needs to be marked as orphaned.
  pure val right_orphans = if (right_outcome.is_updated_or_deleted() and internal_node.right_child != None) {
    Set({
      orphaned_since_version: new_version,
      version: internal_node.right_child.unwrap().version,
      key_hash: right_bits,
    })
  } else Set()
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

In the rust code so far, we had potential mutations:

- Calls to `apply_at_child` can add nodes to the tree
- Calls to `apply_at_child` can add orhpans
- The checks on the block above can add orphans

We set up a result value that include this initial changes, to be used by most cases below. In the cases where we need to make more mutations on top of these, those are added to this initial result value.

```bluespec "definitions" +=
  pure val orphans_from_children =
    left_result.orphans_to_add
      .union(right_result.orphans_to_add)
      .union(left_orphans)
      .union(right_orphans)

  pure val nodes_from_children = left_result.nodes_to_add.union(right_result.nodes_to_add)

  pure val default_result = {
    outcome: Unchanged(None),
    orphans_to_add: orphans_from_children,
    nodes_to_add: nodes_from_children,
  }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Now, lets go case by case on the match expression, which are translated into if expressions in Quint for better readability/correspondence:

```rust
    match (left_outcome, right_outcome) {
```

No match in Quint, as we use ifs instead.

First case:

```rust
       // Neither children is changed. This node is unchanged as well.
       (Outcome::Unchanged(_), Outcome::Unchanged(_)) => {
           Ok(Outcome::Unchanged(Some(Node::Internal(internal_node))))
       },
```

```bluespec "definitions" +=
  if (left_outcome.is_unchanged() and right_outcome.is_unchanged()) {
    // Neither children is changed. This node is unchanged as well.
    { ...default_result, outcome: Unchanged(Some(Internal(internal_node))) }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Second case:

```rust
        // Both children are deleted or never existed. Delete this node as well.
        (
            Outcome::Deleted | Outcome::Unchanged(None),
            Outcome::Deleted | Outcome::Unchanged(None),
        ) => Ok(Outcome::Deleted),
```

```bluespec "definitions" +=
  } else if ((left_outcome == Deleted or left_outcome == Unchanged(None))
              and (right_outcome == Deleted or right_outcome == Unchanged(None))) {
    // Both children are deleted or never existed. Delete this node as well.
    { ...default_result, outcome: Deleted }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Third case:

```rust
        // Left child is a leaf, right child is deleted.
        // Delete the current internal node and move left child up.
        // The child needs to marked as orphaned.
        (Outcome::Updated(left), Outcome::Deleted | Outcome::Unchanged(None))
            if left.is_leaf() =>
        {
            Ok(Outcome::Updated(left))
        },
```

In order to avoid having to do a `match`, we return the `left_outcome` instead of constructing a new `Updated` value. It should be visually evident how this is equivalent to the Rust code:

```bluespec "definitions" +=
  } else if (left_outcome.updated_to_leaf() and (right_outcome == Deleted or right_outcome == Unchanged(None))) {
    // Left child is a leaf, right child is deleted.
    // Delete the current internal node and move left child up.
    // The child needs to marked as orphaned.
    { ...default_result, outcome: left_outcome }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Fourth case:

```rust
        (Outcome::Unchanged(Some(left)), Outcome::Deleted) if left.is_leaf() => {
            // Mark left child as orphaned
            self.mark_node_as_orphaned(
                storage,
                new_version,
                internal_node.left_child.unwrap().version,
                left_bits,
            )?;

            Ok(Outcome::Updated(left))
        },
```

Here, we are forced to `match` in order to extract the node from the outcome to use it in the returned value.

We also may add a new orphan on top of those from the default result.

```bluespec "definitions" +=
  } else if (left_outcome.unchanged_leaf() and right_outcome == Deleted) {
    // Mark left child as orphaned
    match left_outcome {
      | Unchanged(left) => {
        pure val orphans = default_result.orphans_to_add.union(Set({
          orphaned_since_version: new_version,
          version: internal_node.left_child.unwrap().version,
          key_hash: left_bits,
        }))
        { ...default_result, outcome: Updated(left.unwrap()), orphans_to_add: orphans }
      }
      | _ => default_result // impossible
    }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Fifth case: Symmetrical to the third case:

```rust
        // Left child is deleted, right child is a leaf.
        // Delete the current internal node and move right child up.
        // The child needs to marked as orphaned.
        (Outcome::Deleted | Outcome::Unchanged(None), Outcome::Updated(right))
            if right.is_leaf() =>
        {
            Ok(Outcome::Updated(right))
        },
```

```bluespec "definitions" +=
  } else if ((left_outcome == Deleted or left_outcome == Unchanged(None)) and right_outcome.updated_to_leaf()) {
    // Left child is deleted, right child is a leaf.
    // Delete the current internal node and move right child up.
    // The child needs to marked as orphaned.
    { ...default_result, outcome: right_outcome }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Sixth case: Symmetrical to the fourth case

```rust
        (Outcome::Deleted, Outcome::Unchanged(Some(right))) if right.is_leaf() => {
            // Mark right child as orphaned
            self.mark_node_as_orphaned(
                storage,
                new_version,
                internal_node.right_child.unwrap().version,
                right_bits,
            )?;

            Ok(Outcome::Updated(right))
        },
```

```bluespec "definitions" +=
  } else if (left_outcome == Deleted and right_outcome.unchanged_leaf()) {
    // Mark right child as orphaned
    match right_outcome {
      | Unchanged(right) => {
        pure val orphans = default_result.orphans_to_add.union(Set({
          orphaned_since_version: new_version,
          version: internal_node.right_child.unwrap().version,
          key_hash: right_bits,
        }))
        { ...default_result, outcome: Updated(right.unwrap()), orphans_to_add: orphans }
      }
      | _ => default_result // impossible
    }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Seventh case:

```rust
        // At least one child is updated and the path can't be collapsed.
        // Update the currenct node and return
        (left, right) => {
            internal_node.left_child = match left {
                Outcome::Updated(node) => {
                    self.save_node(storage, new_version, left_bits, &node)?;

                    Some(Child {
                        version: new_version,
                        hash: node.hash(),
                    })
                },
                Outcome::Deleted => None,
                Outcome::Unchanged(_) => internal_node.left_child,
            };

            internal_node.right_child = match right {
                Outcome::Updated(node) => {
                    self.save_node(storage, new_version, right_bits, &node)?;

                    Some(Child {
                        version: new_version,
                        hash: node.hash(),
                    })
                },
                Outcome::Deleted => None,
                Outcome::Unchanged(_) => internal_node.right_child,
            };

            Ok(Outcome::Updated(Node::Internal(internal_node)))
        },
    }
}
```

The Quint is similar, apart from not being able to mutate (neither on `save_node` nor on `internal_node.left_child =` and `internal_node.right_child =`), so we use `nodes_to_add` instead.

```bluespec "definitions" +=
  } else {
    // At least one child is updated and the path can't be collapsed.
    // Update the currenct node and return

    pure val new_left_child_and_tree = match left_outcome {
      | Updated(node) => {
        child: Some({ version: new_version, hash: node.hash() }),
        nodes_to_add: Set(({ version: new_version, key_hash: left_bits }, node))
      }
      | Deleted => {
        child: None,
        nodes_to_add: Set(),
      }
      | Unchanged(_) => {
        child: internal_node.left_child,
        nodes_to_add: Set(),
      }
    }

    pure val new_right_child_and_tree = match right_outcome {
      | Updated(node) => {
        child: Some({ version: new_version, hash: node.hash() }),
        nodes_to_add: Set(({ version: new_version, key_hash: right_bits }, node))
      }
      | Deleted => {
        child: None,
        nodes_to_add: Set(),
      }
      | Unchanged(_) => {
        child: internal_node.right_child,
        nodes_to_add: Set(),
      }
    }

    pure val new_internal_node = Internal({
      left_child: new_left_child_and_tree.child,
      right_child: new_right_child_and_tree.child
    })

    {
      ...default_result,
      outcome: Updated(new_internal_node),
      nodes_to_add: default_result.nodes_to_add
        .union(new_left_child_and_tree.nodes_to_add)
        .union(new_right_child_and_tree.nodes_to_add),
    }
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Apply At Child

Applications on children map closely between Rust and Quint.

```rust
fn apply_at_child(
    &self,
    storage: &mut dyn Storage,
    new_version: u64,
    child_bits: BitArray,
    child: Option<Child>,
    batch: Vec<(Hash256, Op<Hash256>)>,
) -> StdResult<Outcome> {
```

```bluespec "definitions" +=
pure def apply_at_child(
  tree: Tree,
  memo: ApplyAtMemo,
  new_version: Version,
  child_bits: BitArray,
  child: Option[Child],
  batch: Set[OperationOnKey]
): ApplyResult = {
```

Once again, Rust uses a `match`, and Quint will use a combination of `if` and `match` instead due to less powerful pattern matching.

```rust
    match (batch.is_empty(), child) {
```

The first two cases in the Rust's `match` are for empty batches (`batch.is_empty` matches `true`):

```rust
        // Child doesn't exist, and there is no op to apply.
        (true, None) => Ok(Outcome::Unchanged(None)),
        // Child exists, but there is no op to apply.
        (true, Some(child)) => {
            let child_node = self.nodes.load(storage, (child.version, &child_bits))?;
            Ok(Outcome::Unchanged(Some(child_node)))
        },
```

```bluespec "definitions" +=
  if (batch == Set()) {
    match child {
      // Child doesn't exist, and there is no op to apply.
      | None => { outcome: Unchanged(None), orphans_to_add: Set(), nodes_to_add: Set() }
      // Child exists, but there is no op to apply.
      | Some(child) => {
        pure val child_node = tree.nodes.get({ version: child.version, key_hash: child_bits })
        { outcome: Unchanged(Some(child_node)), orphans_to_add: Set(), nodes_to_add: Set() }
      }
    }
```

The remaining two cases are for non-empty batches:

```rust
        // Child doesn't exist, but there are ops to apply.
        (false, None) => {
            let (batch, op) = prepare_batch_for_subtree(batch, None);
            debug_assert!(op.is_none());
            self.create_subtree(storage, new_version, child_bits, batch, None)
        },
        // Child exists, and there are ops to apply.
        (false, Some(child)) => {
            self.apply_at(storage, new_version, child.version, child_bits, batch)
        },
    }
}
```

The last non-bracket line is what makes the recursive call to `apply_at`. As explained in [Recursion emulation for `apply_at`](#recursion-emulation-for-apply_at), the Quint version access the value from `memo` instead.

> [!IMPORTANT]
> The `apply_at` call and `memo` access have the same parameters, except for the storage (tree) as discussed before. This is a good indication that the Quint spec replicates the implementation closely, since the value saved to `memo` is the exact call to the Quint `apply_at` definition. The only difference is that it was pre-computed and not computed on the spot at the recursive call.

As a reminder, here's how we previously saved the value to `memo`:

```bluespec
pure val memo_key = (new_version, old_version, bits, batch_here)
pure val memo_value = tree.apply_at(memo, new_version, old_version, bits, batch_here)
memo.put(memo_key, memo_value)
```

```bluespec "definitions" +=
  } else {
    match child {
      // Child doesn't exist, but there are ops to apply.
      | None => {
        pure val batchAndOp = prepare_batch_for_subtree(batch, None)
        tree.create_subtree(new_version, child_bits, batchAndOp._1, None)
      }
      // Child exists, and there are ops to apply.
      | Some(child) => {
        memo.get((new_version, child.version, child_bits, batch))
      }
    }
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Apply at Leaf

Applications on leaves correspondance is similat to that of `apply_at_child`.

```rust
fn apply_at_leaf(
    &self,
    storage: &mut dyn Storage,
    new_version: u64,
    bits: BitArray,
    mut leaf_node: LeafNode,
    batch: Vec<(Hash256, Op<Hash256>)>,
) -> StdResult<Outcome> {
```

```bluespec "definitions" +=
pure def apply_at_leaf(
  tree: Tree,
  new_version: Version,
  bits: BitArray,
  leaf_node: LeafNode,
  batch: Set[OperationOnKey]
): ApplyResult = {
```

We start by preparing the batch:

```rust
    let (batch, op) = prepare_batch_for_subtree(batch, Some(leaf_node));
```

```bluespec "definitions" +=
  pure val batchAndOp = prepare_batch_for_subtree(batch, Some(leaf_node))
  pure val batch = batchAndOp._1
  pure val operation = batchAndOp._2
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

And again, we have a `match`:

```rust
    match (batch.is_empty(), op) {
```

Which again will become a mix of `match`es and `if`s in Quint. The first three scenarios are again for empty batches:

```rust
       (true, Some(Op::Insert(value_hash))) => {
            if value_hash == leaf_node.value_hash {
                // Overwriting with the same value hash, no-op.
                Ok(Outcome::Unchanged(Some(Node::Leaf(leaf_node))))
            } else {
                leaf_node.value_hash = value_hash;
                Ok(Outcome::Updated(Node::Leaf(leaf_node)))
            }
        },
        (true, Some(Op::Delete)) => Ok(Outcome::Deleted),
        (true, None) => Ok(Outcome::Unchanged(Some(Node::Leaf(leaf_node)))),
```

In case of empty batches, we don't need update the nodes nor the orphans. We return the full `ApplyResult` with no updates, since the `else` branch will have updates (as it calls `create_subtree`) and types between both `if` branches (`then` and `else`) need to match.

```bluespec "definitions" +=
  if (batch == Set()) {
    pure val outcome = match operation {
      | Some(op) => {
        match op.op {
          | Insert(value_hash) => {
            if (value_hash == leaf_node.value_hash) {
              // Overwriting with the same value hash, no-op.
              Unchanged(Some(Leaf(leaf_node)))
            } else {
              pure val updated_leaf_node = { ...leaf_node, value_hash: value_hash }
              Updated(Leaf(updated_leaf_node))
            }
          }
          | Delete => Deleted
        }
      }
      | None => Unchanged(Some(Leaf(leaf_node)))
    }

    { outcome: outcome, orphans_to_add: Set(), nodes_to_add: Set() }
```

The last three cases are for non-empty batches:

```rust
        (false, Some(Op::Insert(value_hash))) => {
            leaf_node.value_hash = value_hash;
            self.create_subtree(storage, new_version, bits, batch, Some(leaf_node))
        },
        (false, Some(Op::Delete)) => {
            self.create_subtree(storage, new_version, bits, batch, None)
        },
        (false, None) => {
            self.create_subtree(storage, new_version, bits, batch, Some(leaf_node))
        },
    }
}
```

```bluespec "definitions" +=
  } else {
    match operation {
      | Some(op) => {
        match op.op {
          | Insert(value_hash) => {
              pure val updated_leaf_node = { ...leaf_node, value_hash: value_hash }
              tree.create_subtree(new_version, bits, batch, Some(updated_leaf_node))
          }
          | Delete => {
            tree.create_subtree(new_version, bits, batch, None)
          }
        }
      }
      | None => {
        tree.create_subtree(new_version, bits, batch, Some(leaf_node))
      }
    }
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

## Auxiliary functions

We translated all the apply_* functions already. Now let's look into the functions they use.

### Partition Batch

This is a simple function where Rust uses `partition_point` and `split_off` for the most performatic way to split a vector in two. `bit_at_index` is also a performance-related complexity.

```rust
fn partition_batch<T>(
    mut batch: Vec<(Hash256, T)>,
    bits: BitArray,
) -> (Vec<(Hash256, T)>, Vec<(Hash256, T)>) {
    let partition_point =
        batch.partition_point(|(key_hash, _)| bit_at_index(key_hash, bits.num_bits) == 0);
    let right = batch.split_off(partition_point);
    (batch, right)
}
```

In Quint, we do a simpler partition using a fold and list access for bits.

```bluespec "definitions" +=
pure def partition_batch(
  batch: Set[{ key_hash: BitArray | t }],
  bits: BitArray
): (Set[{ key_hash: BitArray | t }], Set[{ key_hash: BitArray | t }]) = {
  batch.fold((Set(), Set()), (acc, op) => {
    // 0 = left, 1 = right
    if (op.key_hash[bits.length()] == 0) {
      (acc._1.union(Set(op)), acc._2)
    } else {
      (acc._1, acc._2.union(Set(op)))
    }
  })
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Partition Leaf

There is a similar function to partition an optional leaf:

```rust
fn partition_leaf(leaf: Option<LeafNode>, bits: BitArray) -> (Option<LeafNode>, Option<LeafNode>) {
    if let Some(leaf) = leaf {
        let bit = bit_at_index(&leaf.key_hash, bits.num_bits);
        // 0 = left, 1 = right
        debug_assert!(bit == 0 || bit == 1);
        if bit == 0 {
            (Some(leaf), None)
        } else {
            (None, Some(leaf))
        }
    } else {
        (None, None)
    }
}
```

Replacing `if let` with a `match` and `bit_at_index` with simple list access, we have:

```bluespec "definitions" +=
pure def partition_leaf(leaf: Option[LeafNode], bits: BitArray): (Option[LeafNode], Option[LeafNode]) = {
  match leaf {
    | Some(leaf) => {
      // 0 = left, 1 = right
      if (leaf.key_hash[bits.length()] == 0) {
        (Some(leaf), None)
      } else {
        (None, Some(leaf))
      }
    }
    | None => (None, None)
  }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

### Prepare Batch for Subtree

```rust
fn prepare_batch_for_subtree(
    batch: Vec<(Hash256, Op<Hash256>)>,
    existing_leaf: Option<LeafNode>,
) -> (Vec<(Hash256, Hash256)>, Option<Op<Hash256>>) {
```

The return type uses type aliases in Quint as we use records (with named fields) instead of tuples, and types of records are too long to be written inline all the time. The types are equivalent, except from the vector to set difference that was previously discussed.

```bluespec "definitions" +=
pure def prepare_batch_for_subtree(
  batch: Set[OperationOnKey],
  existing_leaf: Option[LeafNode]
): (Set[KeyWithValue], Option[OperationOnKey]) = {
```

```rust
    let mut maybe_op = None;
    let filtered_batch = batch
        .into_iter()
        .filter_map(|(key_hash, op)| {
            // check if key hash match the leaf's
            if let Some(leaf) = existing_leaf {
                if key_hash == leaf.key_hash {
                    maybe_op = Some(op);
                    return None;
                }
            }
            // keep inserts, remove deletes
            if let Op::Insert(value_hash) = op {
                Some((key_hash, value_hash))
            } else {
                None
            }
        })
        .collect();
    (filtered_batch, maybe_op)
}
```

The way to write this in Quint that keeps tighter correspondance is to break down how we find `maybe_op` and `filtered_batch` into two different iterations. This happens because we can't have a mutable `maybe_op` as in Rust, and doing it all on a single iteration would require a more complicated fold. We favor cognitive correspondance over performance.

For `maybe_op`:

```bluespec "definitions" +=
  pure val maybe_op = batch.fold(None, (acc, op) => {
    match existing_leaf {
      | Some(leaf) => {
          if (op.key_hash == leaf.key_hash) {
            Some(op)
          } else {
            acc
          }
        }
      | None => acc
    }
  })
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

When computing `filtered_batch`, Rust has an early return for when it finds an operation matching `existing_leaf`, and we replicate that early return in Quint:

```bluespec "definitions" +=
  pure val filtered_batch = batch.filterMap(op => {
    if (maybe_op == Some(op)) {
      // early return
      None
    } else {
      match op.op {
        | Insert(value_hash) => Some({ key_hash: op.key_hash, value_hash: value_hash })
        | _ => None
      }
    }
  })
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

And finally we return both:

```bluespec "definitions" +=
  (filtered_batch, maybe_op)
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

## Create Subtree

The second recursive function is `create_subtree`, and we also have to emulate it.

### Recursion emulation for `create_subtree`

The Rust signature looks like this:

```rust
fn create_subtree(
    &self,
    storage: &mut dyn Storage,
    version: u64,
    bits: BitArray,
    batch: Vec<(Hash256, Hash256)>,
    existing_leaf: Option<LeafNode>,
) -> StdResult<Outcome> {
```

In Quint, we use pre-computation and memoization:

```bluespec "definitions" +=
pure def create_subtree(
  tree: Tree,
  version: Version,
  bits: BitArray,
  batch: Set[KeyWithValue],
  existing_leaf: Option[LeafNode]
): ApplyResult = {
  pure val memo = pre_compute_create_subtree(tree, version, bits, batch, existing_leaf)

  pure val result = memo.get((version, bits, batch, existing_leaf))
  { outcome: result.outcome, orphans_to_add: Set(), nodes_to_add: result.nodes_to_add }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Let's break that down.

Similar to the `ApplyAtMemo` type, we define a memo type for this function:

```bluespec "definitions" +=
type CreateSubtreeMemo = (Version, BitArray, Set[KeyWithValue], Option[LeafNode]) -> { outcome: Outcome, nodes_to_add: Set[(NodeId, Node)] }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

The pre-computation function receives all paramters and returns this memo:

```bluespec "definitions" +=
pure def pre_compute_create_subtree(
  tree: Tree,
  version: Version,
  bits: BitArray,
  batch: Set[KeyWithValue],
  existing_leaf: Option[LeafNode]
): CreateSubtreeMemo = {
```

Now, for what to pre-compute: we can potentially need to call `create_subtree` recursively for any bit array that has the original `bits` as prefix. Longer bit arrays should be processed before shorter ones, as `create_subtree` for shorter bit arrays may depend on the result of `create_subtree` of longer bit arrays.

`bits_to_compute` defines this list of bit arrays, ordered from shortest to longest length - we will iterate it with `foldr` and therefore start from the right (longest).

```bluespec "definitions" +=
  pure val bits_to_compute = Set(0,1)
    .allListsUpTo(MAX_HASH_LENGTH)
    .filter(b => bits.isPrefixOf(b))
    .toList((a, b) => intCompare(a.length(), b.length()))
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Now the iteration: for each bit array, we can predict with each `batch` and `existing_leaf` it will be called, as that is just a matter of checking prefixes. Similar to `apply_at`, our memo includes all parameters except for the tree/storage itself.

```bluespec "definitions" +=
  bits_to_compute.foldr(Map(), (bits_now, memo) => {
    pure val batch_now = batch.filter(kv => bits_now.isPrefixOf(kv.key_hash))

    pure val existing_leaf_now = if (existing_leaf != None and bits_now.isPrefixOf(existing_leaf.unwrap().key_hash)) {
      existing_leaf
    } else {
      None
    }

    pure val memo_key = (version, bits_now, batch_now, existing_leaf_now)
    pure val memo_value = tree.create_subtree_with_memo(memo, version, bits_now, batch_now, existing_leaf_now)
    memo.put(memo_key, memo_value)
  })
}
```

### Create subtree operator

The pre-computation calls `create_subtree_with_memo`, which is what maps from `create_subtree` in Rust - we already have `create_subtree` in Quint which is responsible for calling the pre-computation and returning the final result, so we gave a different name for this operator:

```bluespec "definitions" +=
pure def create_subtree_with_memo(
  tree: Tree,
  memo: CreateSubtreeMemo,
  version: Version,
  bits: BitArray,
  batch: Set[KeyWithValue],
  existing_leaf: Option[LeafNode]
): { outcome: Outcome, nodes_to_add: Set[(NodeId, Node)] } = {
```

Then we start the translation from the body of `create_subtree` in Rust:

```rust
    match (batch.len(), existing_leaf) {
```

In Quint, this `match` will be if-else branches.

First case:

```rust
        // The subtree to be created is empty: do nothing.
        (0, None) => Ok(Outcome::Unchanged(None)),
```

```bluespec "definitions" +=
   if (batch.size() == 0 and existing_leaf == None) {
     // The subtree to be created is empty: do nothing.
     { outcome: Unchanged(None), nodes_to_add: Set() }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Second case:

```rust
        // The subtree to be created contains exactly one node, which is an
        // existing leaf node.
        (0, Some(leaf_node)) => Ok(Outcome::Unchanged(Some(Node::Leaf(leaf_node)))),
```

```bluespec "definitions" +=
   } else if (batch.size() == 0) {
     // The subtree to be created contains exactly one node, which is an
     // existing leaf node.
     pure val node = Leaf(existing_leaf.unwrap()) // existing_leaf_now is Some for sure, since we didn't match the first case
     { outcome: Updated(node), nodes_to_add: Set() }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Third case:

```rust
        // The subtree to be created contains exactly one node, which is a
        // new leaf node.
        // This case requires special attention: we don't save the node yet,
        // because the path may be collapsed if its sibling gets deleted.
        (1, None) => {
            let (key_hash, value_hash) = only_item(batch);
            Ok(Outcome::Updated(Node::Leaf(LeafNode {
                key_hash,
                value_hash,
            })))
        },
```

```bluespec "definitions" +=
   } else if (batch.size() == 1 and existing_leaf == None) {
     // The subtree to be created contains exactly one node, which is a
     // new leaf node.
     // This case requires special attention: we don't save the node yet,
     // because the path may be collapsed if it's sibling gets deleted.
     pure val node = Leaf(batch.getOnlyElement())
     { outcome: Updated(node), nodes_to_add: Set() }
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Fourth case. Let's break this one into smaller chunks:

First, we call `partition_batch` and `partition_leaf`:

```rust
        // The subtree to be created contains more 2 or more nodes.
        // Recursively create the tree. Return the subtree's root, an
        // internal node.
        // Note that in this scenario, we certainly don't need to collapse the
        // path.
        (_, existing_leaf) => {
            // Split the batch for left and right children.
            let (batch_for_left, batch_for_right) = partition_batch(batch, bits);
            let (leaf_for_left, leaf_for_right) = partition_leaf(existing_leaf, bits);
```

```bluespec "definitions" +=
  } else {
     // The subtree to be created contains more 2 or more nodes.
     // Recursively create the tree. Return the subtree's root, an
     // internal node.
     // Note that in this scenario, we certainly don't need to collapse the
     // path.
     pure val partitioned_batch = partition_batch(batch, bits)
     pure val batch_for_left = partitioned_batch._1
     pure val batch_for_right = partitioned_batch._2

     pure val partitioned_leaf = partition_leaf(existing_leaf, bits)
     pure val leaf_for_left = partitioned_leaf._1
     pure val leaf_for_right = partitioned_leaf._2
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

Then, we make the recursive calls for the left and the right side. In Quint, this means reading from the `memo`:

```rust
            // Create the left subtree.
            let left_bits = bits.extend_one_bit(true);
            let left_outcome = self.create_subtree(
                storage,
                version,
                left_bits,
                batch_for_left,
                leaf_for_left,
            )?;

            // Create the right subtree.
            let right_bits = bits.extend_one_bit(false);
            let right_outcome = self.create_subtree(
                storage,
                version,
                right_bits,
                batch_for_right,
                leaf_for_right,
            )?;
```

```bluespec "definitions" +=
     pure val left_bits = bits.append(0)
     pure val left = memo.get((version, left_bits, batch_for_left, leaf_for_left))

     pure val right_bits = bits.append(1)
     pure val right = memo.get((version, right_bits, batch_for_right, leaf_for_right))
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

> [!IMPORTANT]
> Once again, the `create_subtree` calls and `memo` acceses have the same parameters, except for the storage (tree) as discussed before. This is a good indication that the Quint spec replicates the implementation closely, since the value saved to `memo` is the exact call to the Quint `create_subtree_with_memo` definition (which corresponds to Rust's `create_subtree`). The only difference is that it was pre-computed and not computed on the spot at the recursive call.

Then, we save new nodes. In Quint, this means adding to `nodes_to_add`.

```rust
            // If a subtree is non-empty, save it's root node.
            if let Outcome::Updated(node) | Outcome::Unchanged(Some(node)) = &left_outcome {
                self.save_node(storage, version, left_bits, node)?;
            }
            if let Outcome::Updated(node) | Outcome::Unchanged(Some(node)) = &right_outcome {
                self.save_node(storage, version, right_bits, node)?;
            }

            Ok(Outcome::Updated(Node::Internal(InternalNode {
                left_child: into_child(version, left_outcome),
                right_child: into_child(version, right_outcome),
            })))
        },
    }
}
```

```bluespec "definitions" +=
     pure val nodes_to_add =
       left.nodes_to_add
       .union(right.nodes_to_add)
       .union(match left.outcome {
         | Updated(node) => Set(({ version: version, key_hash: left_bits }, node))
         | Unchanged(option) => match option {
            | Some(node) => Set(({ version: version, key_hash: left_bits }, node))
            | _ => Set()
           }
        | _ => Set()
       })
       .union(match right.outcome {
         | Updated(node) => Set(({ version: version, key_hash: right_bits }, node))
         | Unchanged(option) => match option {
            | Some(node) => Set(({ version: version, key_hash: right_bits }, node))
            | _ => Set()
           }
        | _ => Set()
       })

     pure val node = Internal({
       left_child: into_child(version, left.outcome),
       right_child: into_child(version, right.outcome),
     })

     { outcome: Updated(node), nodes_to_add: nodes_to_add }
   }
}
```
<!-- Empty line, to be tangled but not rendered
```bluespec "definitions" +=

```
-->

And this is all of the main tree manipulation functionality in Rust and Quint.
