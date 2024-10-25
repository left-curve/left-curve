// -*- mode: Bluespec; -*-

module apply_fancy {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import utils.* from "./utils"

  import basicSpells.* from "./spells/basicSpells"
  import commonSpells.* from "./spells/commonSpells"
  import rareSpells.* from "./spells/rareSpells"
  
  /// The return type of apply_* functions, as some of them modify the tree
  type ApplyResult = { outcome: Outcome, tree: Tree }

  // for emulating recursion
  type ApplyAtMemo = (Version, Version, BitArray, Set[OperationOnKey]) -> ApplyResult
  pure val MAX_HEIGHT = 4

  pure val bits_to_compute = Set(0,1)
    .allListsUpTo(MAX_HEIGHT)
    .toList((a, b) => intCompare(a.length(), b.length()))

  pure def sorted_nodes(tree: Tree): List[NodeId] = {
    tree.nodes.keys().toList((a, b) => intCompare(b.key_hash.length(), a.key_hash.length()))
  }

  /// Pre computation of apply_at used to emulate recursion
  pure def pre_compute_apply_at(tree: Tree, new_version: Version, batch: Set[OperationOnKey]): ApplyAtMemo = {
    range(0, new_version + 1).foldl(Map(), (memo, old_version) => {
      bits_to_compute.foldr((memo, tree), (bits, acc) => {
        pure val memo = acc._1
        pure val tree = acc._2

        pure val batch_here = batch.filter(o => bits.isPrefixOf(o.key_hash))

        pure val memo_key = (new_version, old_version, bits, batch_here)
        pure val memo_value = tree.apply_at(memo, new_version, old_version, bits, batch_here)
        (memo.put(memo_key, memo_value), memo_value.tree)
      })._1
    })
  }

  pure def apply(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]) : Tree = {
    pure val tree_1 =
      if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
      } else {
        tree
      }

    pure val memo = pre_compute_apply_at(tree_1, new_version, batch)
    pure val apply_result = tree_1.apply_at(memo, new_version, old_version, ROOT_BITS, batch)

    match apply_result.outcome {
      | Updated(new_root_node) => {
          ...apply_result.tree,
          nodes: apply_result.tree.nodes.put({ version: new_version, key_hash: ROOT_BITS }, new_root_node)
        }
      | _ => apply_result.tree
    }
  }

  pure def apply_at(
    tree: Tree,
    memo: ApplyAtMemo,
    new_version: Version,
    old_version: Version,
    bits: BitArray,
    batch: Set[OperationOnKey]
  ): ApplyResult = {
    if (tree.nodes.has({ version: old_version, key_hash: bits })) {
      match tree.nodes.get({ version: old_version, key_hash: bits }) {
        | Leaf(leaf_node) => tree.apply_at_leaf(new_version, bits, leaf_node, batch)
        | Internal(internal_node) => tree.apply_at_internal(memo, new_version, bits, internal_node, batch)
      }
    } else {
      pure val batchAndOp = prepare_batch_for_subtree(batch, None)
      tree.create_subtree(new_version, bits, batchAndOp._1, None)
    }
  }

  pure def apply_at_internal(
    tree: Tree,
    memo: ApplyAtMemo,
    new_version: Version,
    bits: BitArray,
    internal_node: InternalNode,
    batch: Set[OperationOnKey]
  ): ApplyResult = {
    // Split the batch into two, one for left child, one for right.
    pure val partitioned_batch = q::debug("partitioned batch", partition_batch(batch, q::debug("internal node bits", bits)))
    pure val batch_for_left = partitioned_batch._1
    pure val batch_for_right = partitioned_batch._2

    // Apply the left batch at left child
    pure val left_bits = bits.append(0)
    pure val left_result = tree.apply_at_child(memo, new_version, left_bits, internal_node.left_child, batch_for_left)
    pure val left_outcome = q::debug("left outcome", left_result.outcome)

    // Apply the right batch at right child
    pure val right_bits = bits.append(1)
    pure val right_result = left_result.tree.apply_at_child(memo, new_version, right_bits, internal_node.right_child, batch_for_right)
    pure val right_outcome = q::debug("right outcome", right_result.outcome)

    // If the left child exists and have been updated or deleted, then the
    // old one needs to be marked as orphaned.
    pure val tree_1 = if (left_outcome.is_updated_or_deleted() and internal_node.left_child != None) {
      right_result.tree.mark_node_as_orphaned(new_version, internal_node.left_child.unwrap().version, left_bits)
    } else right_result.tree

    // If the right child exists and have been updated or deleted, then the
    // old one needs to be marked as orphaned.
    pure val tree_2 = if (right_outcome.is_updated_or_deleted() and internal_node.right_child != None) {
      tree_1.mark_node_as_orphaned(new_version, internal_node.right_child.unwrap().version, right_bits)
    } else tree_1

    if (left_outcome.is_unchanged() and right_outcome.is_unchanged()) {
      // Neither children is changed. This node is unchanged as well.
      { outcome: Unchanged(None), tree: tree_2 }

    } else if ((left_outcome == Deleted or left_outcome == Unchanged(None))
                and (right_outcome == Deleted or right_outcome == Unchanged(None))) {
      // Both children are deleted or never existed. Delete this node as well.
      { outcome: Deleted, tree: tree_2 }

    } else if (left_outcome.updated_to_leaf() and (right_outcome == Deleted or right_outcome == Unchanged(None))) {
      // Left child is a leaf, right child is deleted.
      // Delete the current internal node and move left child up.
      // The child needs to marked as orphaned.
       { outcome: left_outcome, tree: tree_2 }

    } else if (left_outcome.unchanged_leaf() and right_outcome == Deleted) {
      // Mark left child as orphaned
      match left_outcome {
        | Unchanged(left) => {
          pure val tree_3 = tree_2.mark_node_as_orphaned(
            new_version,
            internal_node.left_child.unwrap().version,
            left_bits
          )
          { outcome: Updated(left.unwrap()), tree: tree_3 }
        }
        | _ => { outcome: left_outcome, tree: tree_2 } // impossible
      }

    } else if ((left_outcome == Deleted or left_outcome == Unchanged(None)) and right_outcome.updated_to_leaf()) {
      // Left child is deleted, right child is a leaf.
      // Delete the current internal node and move right child up.
      // The child needs to marked as orphaned.
      { outcome: right_outcome, tree: tree_2 }

    } else if (left_outcome == Deleted and right_outcome.unchanged_leaf()) {
      // Mark right child as orphaned
      match right_outcome {
        | Unchanged(right) => {
          pure val tree_3 = tree_2.mark_node_as_orphaned(
            new_version,
            internal_node.right_child.unwrap().version,
            right_bits
          )
          { outcome: Updated(right.unwrap()), tree: tree_3 }
        }
        | _ => { outcome: right_outcome, tree: tree_2 } // impossible
      }

    } else {
      // At least one child is updated and the path can't be collapsed.
      // Update the currenct node and return

      pure val new_left_child_and_tree = match left_outcome {
        | Updated(node) => {
          child: Some({ version: new_version, hash: node.hash() }),
          tree: tree_2.add_node(new_version, left_bits, node)
        }
        | Deleted => {
          child: None,
          tree: tree_2
        }
        | Unchanged(_) => {
          child: internal_node.left_child,
          tree: tree_2
        }
      }

      pure val new_right_child_and_tree = match right_outcome {
        | Updated(node) => {
          child: Some({ version: new_version, hash: node.hash() }),
          tree: new_left_child_and_tree.tree.add_node(new_version, right_bits, node)
        }
        | Deleted => {
          child: None,
          tree: new_left_child_and_tree.tree
        }
        | Unchanged(_) => {
          child: internal_node.right_child,
          tree: new_left_child_and_tree.tree
        }
      }

      pure val new_internal_node = Internal({
        left_child: new_left_child_and_tree.child,
        right_child: new_right_child_and_tree.child
      })

      { outcome: Updated(new_internal_node), tree: new_right_child_and_tree.tree }
    }
  }

  pure def apply_at_child(
    tree: Tree,
    memo: ApplyAtMemo,
    new_version: Version,
    child_bits: BitArray,
    child: Option[Child],
    batch: Set[OperationOnKey]
  ): ApplyResult = {
    if (batch == Set()) {
      match child {
        // Child doesn't exist, and there is no op to apply.
        | None => { outcome: Unchanged(None), tree: tree }
        // Child exists, but there is no op to apply.
        | Some(child) => {
          pure val child_node = tree.nodes.get({ version: child.version, key_hash: child_bits })
          { outcome: Unchanged(Some(child_node)), tree: tree }
        }
      }
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

  pure def apply_at_leaf(
    tree: Tree,
    new_version: Version,
    bits: BitArray,
    leaf_node: LeafNode,
    batch: Set[OperationOnKey]
  ): ApplyResult = {
    pure val batchAndOp = prepare_batch_for_subtree(batch, Some(leaf_node))
    pure val batch = batchAndOp._1
    pure val operation = batchAndOp._2

    if (batch == Set()) {
      pure val outcome = match operation {
        | Some(op) => {
          match op.op {
            | Insert(value_hash) => {
              if (value_hash == leaf_node.value_hash) {
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

      { outcome: outcome, tree: tree }
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

  pure def prepare_batch_for_subtree(
    batch: Set[OperationOnKey],
    existing_leaf: Option[LeafNode]
  ): (Set[KeyWithValue], Option[OperationOnKey]) = {
    // This replicates the filter_map implementation from Rust.
    // It's a bit more complicated as we don't have early returns.
    batch.fold((Set(), None), (acc, op) => {
      pure val batch = acc._1

      pure val is_leaf_op = match existing_leaf {
        | Some(leaf) => {
            if (op.key_hash == leaf.key_hash) {
              Some(op)
            } else {
              acc._2
            }
          }
        | None => acc._2
      }

      if (is_leaf_op != None) {
        // early return
        (batch, is_leaf_op)
      } else {
        match op.op {
          | Insert(value_hash) => (batch.union(Set({ key_hash: op.key_hash, value_hash: value_hash })), None)
          | Delete => (batch, None)
        }
      }
    })
  }

  type CreateSubtreeMemo = BitArray -> { outcome: Outcome, nodes_to_add: Set[(NodeId, Node)] }

  pure def pre_compute_create_subtree(
    tree: Tree,
    version: Version,
    bits: BitArray,
    batch: Set[KeyWithValue],
    existing_leaf: Option[LeafNode]
  ): CreateSubtreeMemo = {
   bits_to_compute.foldr(Map(), (bits_now, memo) => {
     pure val batch_now = batch.filter(kv => bits_now.isPrefixOf(kv.key_hash))

     pure val existing_leaf_now = if (existing_leaf != None and bits_now.isPrefixOf(existing_leaf.unwrap().key_hash)) {
       Some(existing_leaf.unwrap())
     } else {
       None
     }

     pure val result = if (batch_now.size() == 0 and existing_leaf_now == None) {
       // early return
       { outcome: Unchanged(None), nodes_to_add: Set() }
     } else {
       if (batch_now.size() == 0) {
         // existing_leaf_now is Some for sure, since we didn't return early
         pure val node = Leaf(existing_leaf_now.unwrap())
         { outcome: Updated(node), nodes_to_add: Set(({ version: version, key_hash: bits_now }, node)) }

       } else if (batch_now.size() == 1 and existing_leaf_now == None) {
         pure val node = Leaf(batch_now.getOnlyElement())
         { outcome: Updated(node), nodes_to_add: Set(({ version: version, key_hash: bits_now }, node)) }

       } else {
         pure val left = memo.get(bits_now.append(0))
         pure val right = memo.get(bits_now.append(1))

         pure val node = Internal({
           left_child: into_child(version, left.outcome),
           right_child: into_child(version, right.outcome),
         })
         pure val nodes_to_add =
           left.nodes_to_add
           .union(right.nodes_to_add)
           .union(Set(({ version: version, key_hash: bits_now }, node)))

         { outcome: Updated(node), nodes_to_add: nodes_to_add }
       }
     }

      memo.put(bits_now, result)
    })
  }

  pure def create_subtree(
    tree: Tree,
    version: Version,
    bits: BitArray,
    batch: Set[KeyWithValue],
    existing_leaf: Option[LeafNode]
  ): ApplyResult = {
    pure val memo = pre_compute_create_subtree(tree, version, bits, batch, existing_leaf)

    pure val result = memo.get(bits)
    pure val new_nodes = result.nodes_to_add.fold(tree.nodes, (acc, i) => acc.put(i._1, i._2))
    { outcome: result.outcome, tree: { ...tree, nodes: new_nodes } }
  }

  pure def max_height(values_with_key_hash) = {
    values_with_key_hash.fold(0, (max_height, key) => max(max_height, key.key_hash.length()))
  }

  pure def partition_batch(batch: Set[OperationOnKey], bits: BitArray): (Set[OperationOnKey], Set[OperationOnKey]) = {
    batch.fold((Set(), Set()), (acc, op) => {
      // 0 = left, 1 = right
      if (op.key_hash[bits.length()] == 0) {
        (acc._1.union(Set(op)), acc._2)
      } else {
        (acc._1, acc._2.union(Set(op)))
      }
    })
  }

  /// Corresponding save_node, under a different name so it doesn't suggest mutability
  pure def add_node(tree: Tree, version: Version, bits: BitArray, node: Node): Tree = {
    { ...tree, nodes: tree.nodes.put({ version: version, key_hash: bits }, node) }
  }

  pure def into_child(version: Version, outcome: Outcome): Option[Child] = {
    match outcome {
      | Updated(node) => Some({
        version: version,
        hash: node.hash(),
      })
      | Unchanged(_) => None // Unreachable if _ is not None
      | Deleted => None // Unreachable
    }
  }

  pure def is_updated_or_deleted(outcome: Outcome): bool = {
    match outcome {
      | Updated(_) => true
      | Deleted => true
      | _ => false
    }
  }

  pure def is_unchanged(outcome: Outcome): bool = {
    match outcome {
      | Unchanged(_) => true
      | _ => false
    }
  }

  pure def unchanged_leaf(outcome: Outcome): bool = {
    match outcome {
      | Unchanged(optional) => match optional {
        | Some(node) => match node {
          | Leaf(_) => true
          | _ => false
        }
        | _ => false
      }
      | _ => false
    }
  }

  pure def updated_to_leaf(outcome: Outcome): bool = {
    match outcome {
      | Updated(node) => match node {
        | Leaf(_) => true
        | _ => false
      }
      | _ => false
    }
  }
}
