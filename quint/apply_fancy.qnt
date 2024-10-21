// -*- mode: Bluespec; -*-

module apply_fancy {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import basicSpells.* from "./spells/basicSpells"
  import commonSpells.* from "./spells/commonSpells"
  import rareSpells.* from "./spells/rareSpells"

  // for emulating recursion
  type Memo = (Version, Version, BitArray) -> Outcome
  pure val MAX_HEIGHT = 4

  pure def apply_fancy(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]) : Tree = {
    pure val tree_1 = if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
                        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
                      } else {
                        tree
                      }

    pure val max_height_in_tree = tree.nodes.keys().fold(0, (max_height, kv) => {
      max(max_height, kv.key_hash.length())
    })
    pure val full_memo = range(1, max_height_in_tree + 2).foldr(Map(), (i, memo) => {
      pure val node_ids_at_this_height = tree.nodes.keys().filter(k => k.key_hash.length() == MAX_HEIGHT - i)
      node_ids_at_this_height.fold(memo, (memo, node_id) => {
        pure val memo_key = (new_version, old_version, node_id.key_hash)
        pure val memo_value = tree.apply_at(memo, new_version, old_version, node_id.key_hash, batch)
        memo.put(memo_key, memo_value)
        // TODO: it might make sense to also store the parts of the batch relevant to this subtree and then compare to the one being applied in the recursive emulation
      })
    })

    // match full_memo.get((new_version, old_version, ROOT_BITS)) {
    match tree.apply_at(full_memo, new_version, old_version, ROOT_BITS, batch) {
      | Updated(new_root_node) => { ...tree_1, nodes: tree_1.nodes.put({ version: new_version, key_hash: ROOT_BITS }, new_root_node) }
      | _ => tree_1
    }
  }

  pure def apply_at(tree: Tree, memo: Memo, new_version: Version, old_version: Version, bits: BitArray, batch: Set[OperationOnKey]): Outcome = {
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

  pure def apply_at_internal(tree: Tree, memo: Memo, new_version: Version, bits: BitArray, internal_node: InternalNode, batch: Set[OperationOnKey]): Outcome = {
    // TODO
    Deleted
  }

  pure def apply_at_child(tree: Tree, memo: Memo, new_version: Version, child_bits: BitArray, child: Option[Child], batch: Set[OperationOnKey]): Outcome = {
    // TODO
    Deleted
  }

  pure def apply_at_leaf(tree: Tree, new_version: Version, bits: BitArray, leaf_node: LeafNode, batch: Set[OperationOnKey]): Outcome = {
    // TODO
    Deleted
  }

  pure def prepare_batch_for_subtree(batch: Set[OperationOnKey], existing_leaf: Option[LeafNode]): (Set[KeyWithValue], Option[OperationOnKey]) = {
    // This replicates the filter_map implementation from Rust. It's a bit more complicated as we don't have early returns.
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

  pure def create_subtree(tree: Tree, version: Version, bits: BitArray, batch: Set[KeyWithValue], existing_leaf: Option[LeafNode]): Outcome = {
    pure val max_height_in_batch = batch.fold(0, (max_height, kv) => {
      max(max_height, kv.key_hash.length())
    })
    pure val bits_to_compute = Set(0,1).allListsUpTo(max_height_in_batch).toList((a, b) => intCompare(a.length(), b.length()))
    pure val subtrees = bits_to_compute.foldr(Map(), (bits_now, subtrees) => {
      // pure val bits_now = q::debug("bits", if (bits.length() > q::debug("i", i)) bits.slice(0, i) else bits)
      pure val batch_now = batch.filter(kv => bits_now.prefix_of(kv.key_hash))

      // I think here we can figure out in which recursive calls the existing leaf appears.
      // Do we need to?
      pure val existing_leaf_now = if (existing_leaf != None and bits_now.prefix_of(existing_leaf.unwrap().key_hash)) {
        Some(existing_leaf.unwrap())
      } else {
        None
      }

      pure val result = if (batch_now.size() == 0 and existing_leaf_now == None) {
        // early return
        Unchanged(None)
      } else {
        pure val new_node = if (batch_now.size() == 0) {
          // existing_leaf is Some for sure, since we didn't return early
          Leaf(existing_leaf_now.unwrap())
        } else if (batch.size() == 1) {
          Leaf(batch_now.getOnlyElement())
        } else {
          pure val left_outcome = subtrees.getOrElse(bits_now.append(0), Unchanged(None)) // what about the leaf? - we might be able to figure out in which call it appears while precomputing.
          pure val right_outcome = subtrees.getOrElse(bits_now.append(1), Unchanged(None))
          Internal({
            left_child: into_child(version, left_outcome),
            right_child: into_child(version, right_outcome),
          })
        }

        Updated(new_node)
      }

      subtrees.put(bits_now, result)
    })

    subtrees.get(bits)
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
      == { nodes: Map({ key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })), orphans: Set() }
    )

  run multipleOpsOnEmptyTreeTest =
    pure val empty_tree = { nodes: Map(), orphans: Set() }
    pure val ops = Set(
      { key_hash: [0, 1, 1], op: Insert([1, 1, 1, 1]) },
      { key_hash: [0, 1, 0, 0], op: Insert([1, 1, 1, 0]) },
      { key_hash: [1, 1, 1, 1], op: Insert([1, 1, 0, 0]) },
      { key_hash: [1, 1, 1, 0], op: Insert([1, 0, 0, 0]) }
    )
    assert(
      apply_fancy(empty_tree, 0, 1, ops)
      == { nodes: Map({ key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })), orphans: Set() }
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
