// -*- mode: Bluespec; -*-

module apply_super_simple_test {
  import basicSpells.* from "../spells/basicSpells"
  import tree.* from "../tree"
  import node.* from "../node"
  import apply_super_simple.* from "../apply_super_simple"
  pure val empty_tree = { nodes: Map(), orphans: Set() }

  run twoEmptyLeftNodesTest = {
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
       .super_simple_apply(0, 1, { key_hash: [0, 1, 1, 1], value_hash: [5] })
       .super_simple_apply(1, 2, { key_hash: [1, 1, 1, 1], value_hash: [6] })

     pure val result = super_simple_apply(small_tree, 2, 3, { key_hash: [1, 1, 1, 0], value_hash: [7] })

     pure val leaf_for_0 = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [5] })
     pure val expected_leaf_1 = Leaf({ key_hash: [1, 1, 1, 0], value_hash: [7] })
     pure val expected_leaf_2 = Leaf({ key_hash: [1, 1, 1, 1], value_hash: [6] })
     pure val internal_height_3 = Internal({left_child: Some({ version: 3, hash: expected_leaf_1.hash() }),       right_child: Some({ version: 3, hash: expected_leaf_2.hash() })     })
     pure val internal_height_2 = Internal({       left_child: None,       right_child: Some({ version: 3, hash: internal_height_3.hash() })     })
     pure val internal_height_1 = Internal({       left_child: None,       right_child: Some({ version: 3, hash: internal_height_2.hash() })     })
     pure val expected_root = Internal({       left_child: Some({ version: 2, hash: leaf_for_0.hash() }),       right_child: Some({ version: 3, hash: internal_height_1.hash() })     })

     assert(
      result.treeAtVersion(3)
      == Map(
        { key_hash: [], version: 3 } -> expected_root,
        { key_hash: [0], version: 2 } -> leaf_for_0,
        { key_hash: [1], version: 3 } -> internal_height_1,
        { key_hash: [1, 1], version: 3 } -> internal_height_2,
        { key_hash: [1, 1, 1], version: 3 } -> internal_height_3,
        { key_hash: [1, 1, 1, 0], version: 3 } -> expected_leaf_1,
        { key_hash: [1, 1, 1, 1], version: 3 } -> expected_leaf_2
      )       
     )
   }

  run singleInsertOnEmptyTreeTest =
    assert(
      super_simple_apply(empty_tree, 0, 1, { key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })
      == { nodes: Map({ key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })),
           orphans: Set() }
    )
  
  run bigInsertionTest= 
    pure val result = empty_tree
          .super_simple_apply(0, 1, { key_hash: [0, 1, 1], value_hash: [5] })
          .super_simple_apply(1, 2, {key_hash: [1,0,1], value_hash:[77]})
          .super_simple_apply(2, 3, { key_hash: [0, 1, 0], value_hash: [33] })
          .super_simple_apply(3, 4, {key_hash: [1,1,1], value_hash:[42]})

    pure val result_nodes = Map(
          { key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [], version: 2 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 2 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 3 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 4 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 42]), [0, 2] -> Hash, [0] -> Hash), version: 4 })
            }),
          { key_hash: [0, 1, 0], version: 3 } -> Leaf({ key_hash: [0, 1, 0], value_hash: [33] }),
          { key_hash: [0, 1, 1], version: 3 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0, 1], version: 3 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 0, 33]), [0] -> Hash), version: 3 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 3 })
            }),
          { key_hash: [0], version: 2 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0], version: 3 } ->
            Internal({
              left_child: None,
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 0, 1, 0, 33]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 0, 1, 1, 5]), [0, 2] -> Hash, [0] -> Hash), version: 3 })
            }),
          { key_hash: [1, 0], version: 4 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1, 1], version: 4 } -> Leaf({ key_hash: [1, 1, 1], value_hash: [42] }),
          { key_hash: [1], version: 2 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1], version: 4 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 4 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 1, 1, 42]), [0] -> Hash), version: 4 })
            })
    )
    assert(result_nodes == result.nodes)


  

  run bigInsertionUpdateTest = 
    pure val result = empty_tree
          .super_simple_apply(0, 1, { key_hash: [0, 1, 1], value_hash: [5] })
          .super_simple_apply(1, 2, { key_hash: [1,0,1], value_hash:[77] })
          .super_simple_apply(2, 3, { key_hash: [0, 1, 0], value_hash: [33] })
          .super_simple_apply(3, 4, { key_hash: [1,1,1], value_hash:[42] })


    val update = result.super_simple_apply(4, 5, { key_hash: [1,1,1], value_hash:[333] })
    val update1 = update.super_simple_apply(5, 6, { key_hash: [0,1,1], value_hash:[662] })

    pure val result_nodes = Map(
          { key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [], version: 2 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 2 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 3 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 4 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 42]), [0, 2] -> Hash, [0] -> Hash), version: 4 })
            }),
          { key_hash: [0, 1, 0], version: 3 } -> Leaf({ key_hash: [0, 1, 0], value_hash: [33] }),
          { key_hash: [0, 1, 1], version: 3 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0, 1], version: 3 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 0, 33]), [0] -> Hash), version: 3 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 3 })
            }),
          { key_hash: [0], version: 2 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0], version: 3 } ->
            Internal({
              left_child: None,
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 0, 1, 0, 33]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 0, 1, 1, 5]), [0, 2] -> Hash, [0] -> Hash), version: 3 })
            }),
          { key_hash: [1, 0], version: 4 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1, 1], version: 4 } -> Leaf({ key_hash: [1, 1, 1], value_hash: [42] }),
          { key_hash: [1], version: 2 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1], version: 4 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 4 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 1, 1, 42]), [0] -> Hash), version: 4 })
            })
    )
    pure val updated_nodes = Map(
          { key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [], version: 2 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 2 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 3 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 4 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 42]), [0, 2] -> Hash, [0] -> Hash), version: 4 })
            }),
          { key_hash: [], version: 5 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 333]), [0, 2] -> Hash, [0] -> Hash), version: 5 })
            }),
          { key_hash: [0, 1, 0], version: 3 } -> Leaf({ key_hash: [0, 1, 0], value_hash: [33] }),
          { key_hash: [0, 1, 1], version: 3 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0, 1], version: 3 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 0, 33]), [0] -> Hash), version: 3 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 3 })
            }),
          { key_hash: [0], version: 2 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0], version: 3 } ->
            Internal({
              left_child: None,
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 0, 1, 0, 33]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 0, 1, 1, 5]), [0, 2] -> Hash, [0] -> Hash), version: 3 })
            }),
          { key_hash: [1, 0], version: 4 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1, 1], version: 4 } -> Leaf({ key_hash: [1, 1, 1], value_hash: [42] }),
          { key_hash: [1, 1], version: 5 } -> Leaf({ key_hash: [1, 1, 1], value_hash: [333] }),
          { key_hash: [1], version: 2 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1], version: 4 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 4 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 1, 1, 42]), [0] -> Hash), version: 4 })
            }),
          { key_hash: [1], version: 5 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 4 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 1, 1, 333]), [0] -> Hash), version: 5 })
            })
    )
    pure val updated_nodes1 = Map(
          { key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [], version: 2 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 2 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 3 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 2 })
            }),
          { key_hash: [], version: 4 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 42]), [0, 2] -> Hash, [0] -> Hash), version: 4 })
            }),
          { key_hash: [], version: 5 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 5]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 3
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 333]), [0, 2] -> Hash, [0] -> Hash), version: 5 })
            }),
          { key_hash: [], version: 6 } ->
            Internal({
              left_child:
                Some({
                  hash:
                    Map(
                      [0, 0] -> Raw([0]),
                      [0, 1, 0] -> Raw([0]),
                      [0, 1] -> Hash,
                      [0, 2, 0] -> Raw([0]),
                      [0, 2, 1, 0] -> Raw([1, 0, 1, 0, 33]),
                      [0, 2, 1] -> Hash,
                      [0, 2, 2, 0] -> Raw([1, 0, 1, 1, 662]),
                      [0, 2, 2] -> Hash,
                      [0, 2] -> Hash,
                      [0] -> Hash
                    ),
                  version: 6
                }),
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 1, 0, 1, 77]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 1, 1, 1, 333]), [0, 2] -> Hash, [0] -> Hash), version: 5 })
            }),
          { key_hash: [0, 1, 0], version: 3 } -> Leaf({ key_hash: [0, 1, 0], value_hash: [33] }),
          { key_hash: [0, 1, 1], version: 3 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0, 1, 1], version: 6 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [662] }),
          { key_hash: [0, 1], version: 3 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 0, 33]), [0] -> Hash), version: 3 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 5]), [0] -> Hash), version: 3 })
            }),
          { key_hash: [0, 1], version: 6 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 0, 33]), [0] -> Hash), version: 3 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 0, 1, 1, 662]), [0] -> Hash), version: 6 })
            }),
          { key_hash: [0], version: 2 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [5] }),
          { key_hash: [0], version: 3 } ->
            Internal({
              left_child: None,
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 0, 1, 0, 33]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 0, 1, 1, 5]), [0, 2] -> Hash, [0] -> Hash), version: 3 })
            }),
          { key_hash: [0], version: 6 } ->
            Internal({
              left_child: None,
              right_child: Some({ hash: Map([0, 0] -> Raw([0]), [0, 1, 0] -> Raw([1, 0, 1, 0, 33]), [0, 1] -> Hash, [0, 2, 0] -> Raw([1, 0, 1, 1, 662]), [0, 2] -> Hash, [0] -> Hash), version: 6 })
            }),
          { key_hash: [1, 0], version: 4 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1, 1], version: 4 } -> Leaf({ key_hash: [1, 1, 1], value_hash: [42] }),
          { key_hash: [1, 1], version: 5 } -> Leaf({ key_hash: [1, 1, 1], value_hash: [333] }),
          { key_hash: [1], version: 2 } -> Leaf({ key_hash: [1, 0, 1], value_hash: [77] }),
          { key_hash: [1], version: 4 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 4 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 1, 1, 42]), [0] -> Hash), version: 4 })
            }),
          { key_hash: [1], version: 5 } ->
            Internal({
              left_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 0, 1, 77]), [0] -> Hash), version: 4 }),
              right_child: Some({ hash: Map([0, 0] -> Raw([1, 1, 1, 1, 333]), [0] -> Hash), version: 5 })
            })
    )
    assert(all{
      result.nodes == result_nodes,
      update.nodes == updated_nodes,
      update1.nodes == updated_nodes1,
    })


  

  
  run singleInsertOnLeafTest =
    // At first, we have a tree with only "L"
    pure val tree_with_leaf = empty_tree.super_simple_apply(0, 1, { key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] })
    // Then we insert "a"
    pure val result = super_simple_apply(tree_with_leaf, 1, 2, { key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] })

    pure val expected_left_2 = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] }) // L
    pure val expected_right_2 = Leaf({ key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] }) // a
    pure val expected_root_2 = Internal({
      left_child: Some({ version: 2, hash: expected_left_2.hash() }),
      right_child: Some({ version: 2, hash: expected_right_2.hash() })
    })

    assert(
      result.nodes == Map(
           // Old leaf is still here under version 0
           { key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] }),
           // New tree under version 1
           { key_hash: [], version: 2 } -> expected_root_2,
           { key_hash: [0], version: 2 } -> expected_left_2,
           { key_hash: [1], version: 2 } -> expected_right_2
         )
    )
  
  run singleInsertOnLeafThenUpdateTest =
    // At first, we have a tree with only "L"
    pure val tree_with_leaf = empty_tree.super_simple_apply(0, 1, { key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] })
    // Then we insert "a"
    pure val result = super_simple_apply(tree_with_leaf, 1, 2, { key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] })
    // Then we update "a" with new value
    pure val updated_tree_result = super_simple_apply(result, 2, 3, { key_hash: [1, 1, 0, 0], value_hash: [0, 0, 0, 0] })

    pure val expected_left_2 = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] }) // L
    pure val expected_right_2 = Leaf({ key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] }) // a
    pure val expected_root_2 = Internal({
      left_child: Some({ version: 2, hash: expected_left_2.hash() }),
      right_child: Some({ version: 2, hash: expected_right_2.hash() })
    })
    pure val expected_right_3 = Leaf({ key_hash: [1, 1, 0, 0], value_hash: [0, 0, 0, 0] }) // a
    pure val expected_root_3 = Internal({
      left_child: Some({ version: 2, hash: expected_left_2.hash() }),
      right_child: Some({ version: 3, hash: expected_right_3.hash() })
    })
    assert(
      updated_tree_result.nodes == Map(
           // Old leaf is still here under version 1
           { key_hash: [], version: 1 } -> Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 0, 0] }),
           // New tree under version 1
           { key_hash: [], version: 2 } -> expected_root_2,
           { key_hash: [0], version: 2 } -> expected_left_2,
           { key_hash: [1], version: 2 } -> expected_right_2,
           // version 3 after updateing "a"
           { key_hash: [], version: 3 } -> expected_root_3,
           { key_hash: [0], version: 2 } -> expected_left_2,
           { key_hash: [1], version: 3 } -> expected_right_3
         )
    )
}