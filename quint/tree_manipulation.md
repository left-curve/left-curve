# Tree manipulation

This document describes how tree manipulation was modelled in Quint, and how everything corresponds to the Rust implementation. In this version of the Quint model, we tried to make things as close to the Rust implementation as possible. However, Rust and Quint are very different, so there are some challenges:
- The Rust implementation uses recursion, which Quint doesn't support (due to Apalache)
- Rust has mutable variables, while Quint is functional and does not
- Pattern matching in Rust is more powerful than in Quint
- Rust has early returns while Quint is functional and does not
- Rust has statement if while Quint is functional and does not (every `if` needs an `else`).

## Types

Types have a 1:1 mapping between Rust and Quint that is pretty trivial, so won't be covered here. See [tree.qnt](./tree.qnt) and [node.qnt](./node.qnt) for types. The only difference is that we chose to use records instead of tuples for most things, as accessing records is more readable than accessing tuples, specially since Quint doesn't support destructuring tuples at this time.

In order to replicate Rust's mutations over the tree in Quint, we need to also return the modified values somehow. So, we define a type for the return of apply operations:

```bluespec apply_fancy.qnt+=
/// The return type of apply_* functions, as some of them modify the tree
type ApplyResult = { outcome: Outcome, orphans_to_add: Set[OrphanId], nodes_to_add: Set[(NodeId, Node)] }
```


## Recursion emulation for `apply_at`

The Rust functions `apply_at`, `apply_at_internal` and `apply_at_child` are mutually recursive. Since there is no recursion in Quint, we need to emulate it somehow. What happens in the actuall recursion is that longer bit arrays will get computed first, and then smaller bit array computations might depend on the result of longer bit array computations. Therefore, to emulate it, we start from the longest bit arrays, which shouldn't depend on anything else, and save the result in a `memo` value that will be given to what we process next. Then, when a computation calls `apply_at`, we read a result from the `memo` value instead - and the result should already be there, since are computing dependencies first.

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

Recursive calls of `apply_at` include the arguments: `self`, `storage`, `new_version`, `old_version`, `bits` and `batch`. `self` and `storage` are only used to load the tree, which is always done for versions older than the current one being built, so it should be the same for every single call - therefore, we don't make this as part of our key in the `memo` value, as it would introduce significant complexity on figuring out how the tree looked like at that call, and we are not going to read it anyway.

The other fields (`new_version`, `old_version`, `bits` and `batch`) are relevant, and we need to include all of them in the `memo` key so it doesn't end up returning a computation for a full batch while the implementation, at that point, would only have a partial batch. It's easy to predict what batches would be given as arguments to what bits, and we also how the `new_version` from the original call. `bits` can be any of the existing key hashes of the existing tree, and we want to iterate from the longest ones to the shortest ones. We couldn't find a good way to predict `old_version`, so we actually compute `memo` values for all possible versions from `0` to `new_version`, so we know that it will have a value for whatever possible `old_version` it's called with.

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

## Apply operators

### Top level apply

For reference, the Rust code:

```rust
    pub fn apply(
        &self,
        storage: &mut dyn Storage,
        old_version: u64,
        new_version: u64,
        batch: Vec<(Hash256, Op<Hash256>)>,
    ) -> StdResult<Option<Hash256>> {
        // The caller must make sure that versions are strictly incremental.
        // We assert this in debug mode must skip in release to save some time...
        debug_assert!(
            new_version == 0 || new_version > old_version,
            "version is not incremental"
        );

        // If an old root node exists (i.e. tree isn't empty at the old version),
        // mark it as orphaned.
        if self.nodes.has(storage, (old_version, &ROOT_BITS)) {
            self.mark_node_as_orphaned(storage, new_version, old_version, ROOT_BITS)?;
        }

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

The Quint version has a lot of similarities to Rust. Here are the differences:
- Rust has a statement if with a mutation inside it. In Quint, we assign the new value to `tree_1`, and add an `else` branch that returns the unmodified tree. We use `tree_1` everywhere after this.
- We call `pre_compute_apply_at` to prepare for the recursion from calling `apply_at`
- `memo`, the result from the precomputation, is given to `apply_at` so it uses it instead of doing recursion.
- The result obtained from `apply_at` includes mutations to be made to the tree (`nodes_to_add` and `orphans_to_add`). We produce a new tree (`new_tree`) with those changes applied.
- Now, we just need to match, in a less powerful but similar pattern matching. 
- While Rust returns an outcome, Quint actually returns a tree. There are no verification goals for the outcome, so we ignore it. We need to return the tree since there is no storage to fetch the tree from.

```bluespec apply_fancy.qnt+=
  pure def apply(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]): Tree = {
    pure val tree_1 =
      if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
      } else {
        tree
      }

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
