# Tree manipulation

This document describes how tree manipulation was modelled in Quint, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible. However, Rust and Quint are very different, so there are some challenges:
- The Rust implementation uses recursion, which Quint doesn't support (due to Apalache)
- Rust has mutable variables, while Quint is functional and does not
- Pattern matching in Rust is more powerful than in Quint
- Rust has early returns while Quint is functional and does not
- Rust has statement if while Quint is functional and does not (every `if` needs an `else`).

## Types

Types have a 1:1 mapping between Rust and Quint that is pretty trivial, so won't be covered here. See [tree.qnt](./tree.qnt) and [node.qnt](./node.qnt) for types. The only difference is that we chose to use records instead of tuples for most things, as accessing records is more readable than accessing tuples, specially since Quint doesn't support destructuring tuples at this time.

In order to replicate Rust's mutations over the tree in Quint, we need to also return the modified values. So, we define a type for the return of apply operations:

```bluespec apply_fancy.qnt+=
/// The return type of apply_* functions, as some of them modify the tree
type ApplyResult = { outcome: Outcome, orphans_to_add: Set[OrphanId], nodes_to_add: Set[(NodeId, Node)] }
```


## Recursion emulation for `apply_at`

The Rust functions `apply_at`, `apply_at_internal` and `apply_at_child` are mutually recursive. Since there is no recursion in Quint, we need to emulate it. What happens in the actual recursion is that longer bit arrays will get computed first, and then smaller bit array computations might depend on the result of longer bit array computations. Therefore, to emulate it, we start from the longest bit arrays, which shouldn't depend on anything else, and save the result in a `memo` value that will be given to what we process next. Then, when a computation calls `apply_at`, we read a result from the `memo` value instead - and the result should already be there, since are computing dependencies first.

```rust
    fn apply_at(
        &self,
        storage: &mut dyn Storage,
        new_version: u64,
        old_version: u64,
        bits: BitArray,
        batch: Vec<(Hash256, Op<Hash256>)>,
    ) -> StdResult<Outcome> 
```

```bluespec apply_fancy.qnt+=
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

#### Concrete example

Consider the following tree:
```
       root
     ┌──┴──┐
    (0)    1
  ┌──┴──┐
 00    (01)
     ┌──┴──┐
    010   011
```

With `010` being a prefix for `0100` and `011` being a prefix `0110`. Consider the following batch:
```
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

> [!TIP]
> There might be a way to predict `old_version` for each call and avoid unnecessary computations. We should look into this in the future. This does *not* affect correctness, only performance.

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

```bluespec apply_fancy.qnt+=
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

```bluespec apply_fancy.qnt+=
    pure val tree_1 =
      if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
      } else {
        tree
      }
```

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

```bluespec apply_fancy.qnt+=
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

## Apply At

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

```bluespec apply_fancy.qnt+=
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

## Apply At Internal

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

```bluespec apply_fancy.qnt+=
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

```bluespec apply_fancy.qnt+=
    // Apply the left batch at left child
    pure val left_bits = bits.append(0)
    pure val left_result = tree.apply_at_child(memo, new_version, left_bits, internal_node.left_child, batch_for_left)
    pure val left_outcome = left_result.outcome

    // Apply the right batch at right child
    pure val right_bits = bits.append(1)
    pure val right_result = tree.apply_at_child(memo, new_version, right_bits, internal_node.right_child, batch_for_right)
    pure val right_outcome = right_result.outcome
```

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

```bluespec apply_fancy.qnt+=
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

In the rust code so far, we had potential mutations:
- Calls to `apply_at_child` can add nodes to the tree
- Calls to `apply_at_child` can add orhpans
- The checks on the block above can add orphans

We set up a result value that include this initial changes, to be used by most cases below. In the cases where we need to make more mutations on top of these, those are added to this initial result value. 

```bluespec apply_fancy.qnt+=
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

```bluespec apply_fancy.qnt+=
    if (left_outcome.is_unchanged() and right_outcome.is_unchanged()) {
      // Neither children is changed. This node is unchanged as well.
      { ...default_result, outcome: Unchanged(Some(Internal(internal_node))) }
```

Second case:

```rust
            // Both children are deleted or never existed. Delete this node as well.
            (
                Outcome::Deleted | Outcome::Unchanged(None),
                Outcome::Deleted | Outcome::Unchanged(None),
            ) => Ok(Outcome::Deleted),
```

```bluespec apply_fancy.qnt+=
    } else if ((left_outcome == Deleted or left_outcome == Unchanged(None))
                and (right_outcome == Deleted or right_outcome == Unchanged(None))) {
      // Both children are deleted or never existed. Delete this node as well.
      { ...default_result, outcome: Deleted }
```

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

```bluespec apply_fancy.qnt+=
    } else if (left_outcome.updated_to_leaf() and (right_outcome == Deleted or right_outcome == Unchanged(None))) {
      // Left child is a leaf, right child is deleted.
      // Delete the current internal node and move left child up.
      // The child needs to marked as orphaned.
      { ...default_result, outcome: left_outcome }
```

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

```bluespec apply_fancy.qnt+=
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

Fifth case: Symmetrical to the Third case:

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


```bluespec apply_fancy.qnt+=
    } else if ((left_outcome == Deleted or left_outcome == Unchanged(None)) and right_outcome.updated_to_leaf()) {
      // Left child is deleted, right child is a leaf.
      // Delete the current internal node and move right child up.
      // The child needs to marked as orphaned.
      { ...default_result, outcome: right_outcome }
```

Sixth case: Symmetrical to the Fourth case

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

```bluespec apply_fancy.qnt+=
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

```bluespec apply_fancy.qnt+=
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
