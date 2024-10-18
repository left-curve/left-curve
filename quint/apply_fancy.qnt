// -*- mode: Bluespec; -*-

module apply_fancy {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import basicSpells.* from "./spells/basicSpells"

  // for emulating recursion
  type Memo = (Version, Version, BitArray) -> Outcome
  pure val MAX_HEIGHT = 4

  pure def apply_fancy(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]) : Tree = {
    pure val tree_1 = if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
                        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
                      } else {
                        tree
                      }

    pure val full_memo = range(1, MAX_HEIGHT).foldr(Map(), (i, memo) => {
      pure val node_ids_at_this_height = tree.nodes.keys().filter(k => k.key_hash.length() == MAX_HEIGHT - i)
      node_ids_at_this_height.fold(memo, (memo, node_id) => {
        pure val memo_key = (new_version, old_version, node_id.key_hash)
        pure val memo_value = tree.apply_at(memo, new_version, old_version, node_id.key_hash, batch)
        memo.put(memo_key, memo_value)
        // TODO: it might make sense to also store the parts of the batch relevant to this subtree and then compare to the one being applied in the recursive emulation
      })
    })
    // TODO: handle outcomes
    tree_1
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
    pure val subtrees = range(bits.length() + 1, max_height_in_batch).foldr(Map(), (i, subtrees) => {
      pure val filtered_batch = batch.filter(kv => kv.key_hash.length() == i)

      // I think here we can figure out in which recursive calls the existing leaf appears.
      // Do we need to?
      // Anyway, this is not exaclty the code for that since it will appear in all calls leading to this exact call
      pure val existing_leaf_for_this = if (existing_leaf != None and existing_leaf.unwrap().key_hash.length() == i) {
        Some(existing_leaf.unwrap())
      } else {
        None
      }

      // TODO: hard part - I think we can move the code below into here, or we might need a copy of it.
      subtrees
    })

    if (batch.size() == 0 and existing_leaf == None) {
      // early return
      Unchanged(None)
    } else {
      pure val new_node = if (batch.size() == 0) {
        // existing_leaf is Some for sure, since we didn't return early
        Leaf(existing_leaf.unwrap())
      } else if (batch.size() == 1) {
        Leaf(batch.getOnlyElement())
      } else {
        pure val left_outcome = subtrees.get(bits.append(0)) // what about the leaf? - we might be able to figure out in which call it appears while precomputing.
        pure val right_outcome = subtrees.get(bits.append(1))
        Internal({
          left_child: into_child(version, left_outcome),
          right_child: into_child(version, right_outcome),
        })
      }

      Updated(new_node)
    }
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
}
