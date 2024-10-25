// -*- mode: Bluespec; -*-

module super_simple_apply {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import utils.* from "./utils"
  import basicSpells.* from "./spells/basicSpells"

  pure def apply(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]): Tree = {
    batch.fold(tree, (t, op) => {
      match op.op {
        | Insert(value_hash) => {
          pure val newLeaf = { key_hash: op.key_hash, value_hash: value_hash }
          super_simple_apply(t, old_version, new_version, newLeaf)
        }
        | Delete => t // TODO: implement delete
      }
    })
  }

  pure def super_simple_apply(tree: Tree, old_version: Version, new_version: Version, insert: LeafNode): Tree = {
    pure val existing_keys_with_values = tree.nodes.keys().fold(Set(), (acc, k) => {
      if (k.version == old_version) {
        match tree.nodes.get(k) {
          | Leaf(leaf) => acc.union(Set(leaf))
          | _ => acc
        }
      } else {
        acc
      }
    })

    pure val keys_with_values = existing_keys_with_values.union(Set(insert))
    pure val result = range(0, MAX_HASH_LENGTH).foldr((Set(), keys_with_values.map(kv => { key_hash: kv.key_hash, node: Leaf(kv) })), (i, acc) => {
      acc._2.fold(acc, (acc, kv) => {
        pure val prefix = kv.key_hash.slice(0, i)
        pure val left = acc._2.find(k => k.key_hash == prefix.append(0))
        pure val right = acc._2.find(k => k.key_hash == prefix.append(1))
        pure val queue_1 = acc._2.filter(k => k.key_hash != prefix.append(0) and k.key_hash != prefix.append(1))
        pure val queue_2 = if (left != None and right != None) {
          pure val internal_node = Internal({
            left_child: Some({ version: new_version, hash: left.unwrap().node.hash() }),
            right_child: Some({ version: new_version, hash: right.unwrap().node.hash() })
          })
          // old queue, don't pop
          acc._2.union(Set({ key_hash: prefix, node: internal_node }))
        } else if (left != None) {
          match left.unwrap().node {
            | Internal(_) => {
               pure val internal_node = Internal({
                 left_child: Some({ version: new_version, hash: left.unwrap().node.hash() }),
                 right_child: None,
               })
               // old queue, don't pop
               acc._2.union(Set({ key_hash: prefix, node: internal_node }))
            }
            | Leaf(leaf) => queue_1.union(Set({ key_hash: prefix, node: left.unwrap().node }))
          }
        } else if (right != None) {
          match right.unwrap().node {
            | Internal(_) => {
               pure val internal_node = Internal({
                 left_child: None,
                 right_child: Some({ version: new_version, hash: right.unwrap().node.hash() })
               })
               // old queue, don't pop
               acc._2.union(Set({ key_hash: prefix, node: internal_node }))
            }
            | Leaf(leaf) => queue_1.union(Set({ key_hash: prefix, node: right.unwrap().node }))
          }
        } else if (i == 0) {
          pure val root = queue_1.getOnlyElement()
          acc._2.union(Set({ key_hash: prefix, node: root.node }))
        } else {
          acc._2
        }
        (acc._1.union(queue_2), queue_2)
      })
    })

    { nodes: result._2.map(kv => ({version: new_version, key_hash: kv.key_hash}, kv.node)).setToMap(), orphans: Set() }
  }

  pure val empty_tree = { nodes: Map(), orphans: Set() }
   run simple::SingleInsertOnLeafTest =
    // At first, we have a tree with only "L"
    pure val tree_with_leaf = super_simple_apply(empty_tree, 0, 0, { key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] })

    // Then we insert "a"
    pure val result = super_simple_apply(tree_with_leaf, 0, 1, { key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] })

    pure val expected_left = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] }) // L
    pure val expected_right = Leaf({ key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] }) // a
    pure val expected_root = Internal({
      left_child: Some({ version: 1, hash: expected_left.hash() }),
      right_child: Some({ version: 1, hash: expected_right.hash() })
    })

    assert(
      result.nodes == Map(
           // Old leaf is still here under version 0
           // TODO: add versions on simple
           // { key_hash: [], version: 0 } -> Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] }),
           // New tree under version 1
           { key_hash: [], version: 1 } -> expected_root,
           { key_hash: [0], version: 1 } -> expected_left,
           { key_hash: [1], version: 1 } -> expected_right
         )
    )
    // TODO: add orphans expectation
    // orphans: Set({ key_hash: [], orphaned_since_version: 1, version: 0 }) }


  run simple::singleInsertOnEmptyTreeTest =
    assert(
      super_simple_apply(empty_tree, 0, 1, { key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })
      == { nodes: Map({ key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })),
           orphans: Set() }
    )

   run simple::aleksandarsTest = {
     //           root
     //         ┌──┴──┐
     //        (0)    1
     //            ┌──┴──┐
     //          null   (11)
     //               ┌──┴──┐
     //             null  (111)
     //                  ┌──┴──┐
     //                1110   1111

     pure val small_tree = empty_tree
       .super_simple_apply(0, 0, { key_hash: [0, 1, 1, 1], value_hash: [5] })
       .super_simple_apply(0, 0, { key_hash: [1, 1, 1, 1], value_hash: [6] })

     pure val result = super_simple_apply(small_tree, 0, 1, { key_hash: [1, 1, 1, 0], value_hash: [7] })

     pure val leaf_for_0 = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [5] })
     pure val expected_leaf_1 = Leaf({ key_hash: [1, 1, 1, 0], value_hash: [7] })
     pure val expected_leaf_2 = Leaf({ key_hash: [1, 1, 1, 1], value_hash: [6] })
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
       left_child: Some({ version: 1, hash: leaf_for_0.hash() }),
       right_child: Some({ version: 1, hash: internal_height_1.hash() }),
     })

     assert(
       result
       == { nodes: Map(
            { key_hash: [], version: 1 } -> expected_root,
            { key_hash: [0], version: 1 } -> leaf_for_0,
            { key_hash: [1], version: 1 } -> internal_height_1,
            { key_hash: [1, 1], version: 1 } -> internal_height_2,
            { key_hash: [1, 1, 1], version: 1 } -> internal_height_3,
            { key_hash: [1, 1, 1, 0], version: 1 } -> expected_leaf_1,
            { key_hash: [1, 1, 1, 1], version: 1 } -> expected_leaf_2
          ), orphans: Set() }
     )
   }
}
