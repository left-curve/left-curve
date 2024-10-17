// -*- mode: Bluespec; -*-

module apply_fancy {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import basicSpells.* from "./spells/basicSpells"

  pure def apply_fancy(tree: Tree, old_version: Version, new_version: Version, batch: List[OperationOnBytes]) : Tree = {
    pure val tree_1 = if (tree.nodes.has({ version: old_version, key_hash: ROOT_BITS })) {
                        tree.mark_node_as_orphaned(new_version, old_version, ROOT_BITS)
                      } else {
                        tree
                      }

    pure val outcome = tree.apply_at(new_version, old_version, ROOT_BITS, batch)
    // TODO: handle outcomes
    tree_1
  }

  pure def apply_at(tree: Tree, new_version: Version, old_version: Version, bits: BitArray, batch: List[OperationOnBytes]): Outcome = {
    if (tree.nodes.has({ version: old_version, key_hash: bits })) {
      match tree.nodes.get({ version: old_version, key_hash: bits }) {
        | Leaf(leaf_node) => tree.apply_at_leaf(new_version, bits, leaf_node, batch)
        | Internal(internal_node) => tree.apply_at_internal(new_version, bits, internal_node, batch)
      }
    } else {
      pure val batchAndOp = prepare_batch_for_subtree(batch, None)
      tree.create_subtree(new_version, bits, batch, None)
    }
  }

  pure def apply_at_internal(tree: Tree, new_version: Version, bits: BitArray, internal_node: InternalNode, batch: List[OperationOnBytes]): Outcome = {
    // TODO
    Deleted
  }

  pure def apply_at_child(tree: Tree, new_version: Version, child_bits: BitArray, child: Option[Child], batch: List[OperationOnBytes]): Outcome = {
    // TODO
    Deleted
  }

  pure def apply_at_leaf(tree: Tree, new_version: Version, bits: BitArray, leaf_node: LeafNode, batch: List[OperationOnBytes]): Outcome = {
    // TODO
    Deleted
  }

  pure def prepare_batch_for_subtree(batch: List[OperationOnBytes], existing_leaf: Option[LeafNode]): (List[OperationOnBytes], Option[OperationOnBytes]) = {
    // TODO
    (batch, None)
  }

  pure def create_subtree(tree: Tree, version: Version, bits: BitArray, batch: List[OperationOnBytes], existing_leaf: Option[LeafNode]): Outcome = {
    // TODO
    Deleted
  }
}
