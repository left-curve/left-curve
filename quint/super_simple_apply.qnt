// -*- mode: Bluespec; -*-

module super_simple_apply {
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import utils.* from "./utils"
  import basicSpells.* from "./spells/basicSpells"
  import invariants.* from "./invariants"
  pure val MAX_HEIGHT = 4
  
  pure def orphanRootIfExists(tree: Tree, old_version: Version, new_version: Version): Tree = 
    if(tree.nodes.keys().size() == 0 ) tree else {...tree, orphans: tree.orphans
                                                      .union(Set(makeOrphanId(new_version, old_version, ROOT_BITS)))}

  pure def cmpInternalAndNode(internal : InternalNode, internalOldVersion: InternalNode) : bool = {
    val newLeftUnwrapped = match internal.left_child {
      | Some(left_child) => left_child.hash
      | None => Map()
    }
    val newRightUnwrapped = match internal.right_child {
      | Some(right_child) => right_child.hash
      | None => Map()
    }
    
    val oldLeftUnwrapped = match internalOldVersion.left_child {
      | Some(left_child) => left_child.hash
      | None => Map()
    }
    val oldRightUnwrapped = match internalOldVersion.right_child {
      | Some(right_child) => right_child.hash
      | None => Map()
    }
    if(not(newLeftUnwrapped==Map())
      and not(newRightUnwrapped==Map())
      and not(oldLeftUnwrapped==Map())
      and not(oldRightUnwrapped==Map()))
      newLeftUnwrapped == oldLeftUnwrapped and newRightUnwrapped == oldRightUnwrapped
    else
      false
  }

  pure def removeDuplicateEntries(nodes: NodeId->Node, new_version: Version) : NodeId->Node =
    nodes.keys().fold(nodes, (nodes, key) => 
      val nodeForKey = nodes.get(key)
      if(nodes.keys().contains({...key, version: new_version})) 
        val nodeForKeyWithNewVersion = nodes.get({...key, version: new_version})
        if(nodeForKey == nodeForKeyWithNewVersion and not(new_version==key.version))
          nodes.mapRemove({...key, version: new_version})
        else     
          //  match nodeForKeyWithNewVersion {
          //    | Internal(internalNewVersion) 
          //       => match nodeForKey {
          //         | Internal(internalOldVersion) =>  
          //           if(cmpInternalAndNode(internalNewVersion, internalOldVersion)) 
          //             nodes.mapRemove({...key, version: new_version})
          //           else nodes
          //         | Leaf(_) => nodes
          //       }
          //    | Leaf(_) => nodes
          //  }
          nodes
      else 
        nodes
    )
      
      
  pure def super_simple_apply(tree: Tree, old_version: Version, new_version: Version, insert: LeafNode): Tree = {
    
    val treeWithOrphanedRoot = orphanRootIfExists(tree, old_version, new_version)
    val treeForOldVersion = treeAtVersion(tree, old_version)
    pure val existing_keys_with_values = treeForOldVersion.keys().fold(Set(), (acc, k) => {
        match tree.nodes.get(k) {
          | Leaf(leaf) => acc.union(Set(leaf))
          | _ => acc
        }
    })

    pure val keys_with_values = existing_keys_with_values.union(Set(insert))
    pure val result = range(0, MAX_HEIGHT).foldr((Set(), keys_with_values.map(kv => { key_hash: kv.key_hash, node: Leaf(kv) })), (i, acc) => {
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
    
    val nodes = result._2.map(kv => ({version: new_version, key_hash: kv.key_hash}, kv.node)).fold(tree.nodes, (nodes, kv) => nodes.put(kv._1, kv._2))

    val newNodes = removeDuplicateEntries(nodes, new_version)
    { nodes: newNodes, orphans: treeWithOrphanedRoot.orphans }
  }
  run simple::singleInsertOnEmptyTreeTest_A =
    pure val t1 = super_simple_apply(empty_tree, 0, 1, { key_hash: [0, 1, 1], value_hash: [5] })
    pure val t2 = super_simple_apply(t1, 1, 2, { key_hash: [0, 0, 1], value_hash: [33] })
    pure val t3 = super_simple_apply(t2, 2, 3, {key_hash: [1,0,1], value_hash:[77]})
    pure val t4 = super_simple_apply(t3, 3, 4, {key_hash: [1,1,1], value_hash:[42]})

    assert(true==true)

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
