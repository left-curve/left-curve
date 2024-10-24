// -*- mode: Bluespec; -*-

module apply_fancy {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import basicSpells.* from "./spells/basicSpells"
  import commonSpells.* from "./spells/commonSpells"
  import rareSpells.* from "./spells/rareSpells"

  /// The return type of apply_* functions, as some of them modify the tree
  type ApplyResult = { outcome: Outcome, tree: Tree }

  // for emulating recursion
  type ApplyAtMemo = (Version, Version, BitArray) -> ApplyResult
  pure val MAX_HEIGHT = 4


  /* Pre-computations to emulate recursion */

  pure def pre_compute_apply_at(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]): ApplyAtMemo = {
    range(1, tree.nodes.keys().max_height() + 2).foldr(Map(), (i, memo) => {
      pure val node_ids_at_this_height = tree.nodes.keys().filter(k => k.key_hash.length() == MAX_HEIGHT - i)

      node_ids_at_this_height.fold(memo, (memo, node_id) => {
        pure val memo_key = (new_version, old_version, node_id.key_hash)
        // TODO: it might make sense to also store the parts of the batch relevant to this subtree and then compare to
        // the one being applied in the recursive emulation
        pure val memo_value = tree.apply_at(memo, new_version, old_version, node_id.key_hash, batch)
        memo.put(memo_key, memo_value)
      })
    })
  }

  pure def apply_fancy(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]) : Tree = {
    pure val tree_1 =
      if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
      } else {
        tree
      }

    pure val memo = pre_compute_apply_at(tree, old_version, new_version, batch)
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

  pure def apply_at_internal(tree: Tree, memo: ApplyAtMemo, new_version: Version, bits: BitArray, internal_node: InternalNode,
                             batch: Set[OperationOnKey]): ApplyResult = {
    // TODO
    { outcome: Deleted, tree: tree }
  }

  pure def apply_at_child(tree: Tree, memo: ApplyAtMemo, new_version: Version, child_bits: BitArray, child: Option[Child],
                          batch: Set[OperationOnKey]): ApplyResult = {
    // TODO
    { outcome: Deleted, tree: tree }
  }

  pure def apply_at_leaf(tree: Tree, new_version: Version, bits: BitArray, leaf_node: LeafNode,
                         batch: Set[OperationOnKey]): ApplyResult = {
    // TODO
    { outcome: Deleted, tree: tree }
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
              None
            }
          }
        | None => None
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
    pure val bits_to_compute = Set(0,1)
      .allListsUpTo(batch.max_height())
      .toList((a, b) => intCompare(a.length(), b.length()))

   bits_to_compute.foldr(Map(), (bits_now, memo) => {
     pure val batch_now = batch.filter(kv => bits_now.prefix_of(kv.key_hash))

     pure val existing_leaf_now = if (existing_leaf != None and bits_now.prefix_of(existing_leaf.unwrap().key_hash)) {
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

  run singleInsertOnEmptyTreeTest =
    pure val empty_tree = { nodes: Map(), orphans: Set() }
    assert(
      apply_fancy(empty_tree, 0, 1, Set({ key_hash: [0, 1, 1], op: Insert([1, 1, 1, 1]) }))
      == { nodes: Map({ key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })),
           orphans: Set() }
    )

  run twoInsertsOnEmptyTreeTest =
    pure val empty_tree = { nodes: Map(), orphans: Set() }
    pure val ops = Set(
      { key_hash: [1, 1, 0, 0], op: Insert([1, 1, 1, 1]) }, // a
      { key_hash: [0, 1, 0, 0], op: Insert([1, 1, 1, 0]) } // r
    )

    pure val expected_left = Leaf({ key_hash: [0, 1, 0, 0], value_hash: [1, 1, 1, 0] })
    pure val expected_right = Leaf({ key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] })
    pure val expected_root = Internal({
      left_child: Some({ version: 1, hash: expected_left.hash() }),
      right_child: Some({ version: 1, hash: expected_right.hash() })
    })
    assert(
      apply_fancy(empty_tree, 0, 1, ops)
      == { nodes: Map(
           { key_hash: [], version: 1 } -> expected_root,
           { key_hash: [0], version: 1 } -> expected_left,
           { key_hash: [1], version: 1 } -> expected_right
         ), orphans: Set() }
    )

  run twoInsertsWihSharedPrefixOnEmptyTreeTest =
    pure val empty_tree = { nodes: Map(), orphans: Set() }
    pure val ops = Set(
      { key_hash: [0, 1, 1, 0], op: Insert([1, 1, 1, 1]) }, // m
      { key_hash: [0, 1, 1, 1], op: Insert([1, 1, 1, 0]) } // L
    )

    pure val expected_leaf_1 = Leaf({ key_hash: [0, 1, 1, 0], value_hash: [1, 1, 1, 1] })
    pure val expected_leaf_2 = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 1 ,0] })
    pure val internal_height_3 = Internal({
      left_child: Some({ version: 1, hash: expected_leaf_1.hash() }),
      right_child: Some({ version: 1, hash: expected_leaf_2.hash() })
    })
    pure val internal_height_2 = Internal({
      left_child: None,
      right_child: Some({ version: 1, hash: internal_height_3.hash() })
    })
    pure val internal_height_1 = Internal({
      left_child: None,
      right_child: Some({ version: 1, hash: internal_height_2.hash() })
    })
    pure val expected_root = Internal({
      left_child: Some({ version: 1, hash: internal_height_1.hash() }),
      right_child: None
    })

    assert(
      apply_fancy(empty_tree, 0, 1, ops)
      == { nodes: Map(
           { key_hash: [], version: 1 } -> expected_root,
           { key_hash: [0], version: 1 } -> internal_height_1,
           { key_hash: [0, 1], version: 1 } -> internal_height_2,
           { key_hash: [0, 1, 1], version: 1 } -> internal_height_3,
           { key_hash: [0, 1, 1, 0], version: 1 } -> expected_leaf_1,
           { key_hash: [0, 1, 1, 1], version: 1 } -> expected_leaf_2
         ), orphans: Set() }
    )

  // HACKS: these should be added to Quint soon

  pure def unwrap(value: Option[a]): a = {
    match value {
      | None => Map().get(value)
      | Some(x) => x
    }
  }

  pure def getOnlyElement(s: Set[a]): a = {
    pure val hack = s.map(e => ("value", e)).setToMap()
    if (s.size() != 1) {
      hack.get("error: expected singleton")
    } else {
      hack.get("value")
    }
  }

  pure def prefix_of(l1: List[a], l2: List[b]): bool = and {
    l1.length() <= l2.length(),
    l1.indices().forall(i => l1[i] == l2[i])
  }
}
