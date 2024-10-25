// -*- mode: Bluespec; -*-

module apply_simple_test {
  import basicSpells.* from "../spells/basicSpells"
  import tree.* from "../tree"
  import node.* from "../node"
  import apply_simple.* from "../apply_simple"

  pure def makeNodeId(v: Version, kh: BitArray): NodeId =
    { version: v, key_hash: kh }

  run CommonPrefixTest1 = {
    val testTree : Tree = {
      orphans: Set(),nodes: Map(
      // root node without hash pointing to left internal node and right leaf node
      makeNodeId(0, []) -> Internal({
        left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
        right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
      }),
      makeNodeId(0, [0]) -> Internal({
        left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
        right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
      }),
      makeNodeId(0, [1]) -> Leaf({value_hash: [0], key_hash:[0]}),
        makeNodeId(0, [0,0]) -> Internal({
          left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
          right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
        }),
          makeNodeId(0, [0,0,0]) -> Leaf({value_hash: [0], key_hash:[0,0,0,0]}),
          makeNodeId(0, [0,0,1]) -> Leaf({value_hash: [0], key_hash:[0,0,1,0]}),
        makeNodeId(0, [0,1]) -> Internal({
          left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
          right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
        }),
          makeNodeId(0, [0,1,0]) -> Leaf({value_hash: [0], key_hash:[0,1,0,0]}),
          makeNodeId(0, [0,1,1]) -> Leaf({value_hash: [0], key_hash:[0,1,1,0]})
    )}
    val keyValue = {key_hash : [0,1,1,0], value_hash:[1]}
    val longestPrefixNode = getKeyWithLongestOverlappingPrefix(testTree, keyValue)
    assert({key_hash : [0,1], version:0} == longestPrefixNode)
  }

  run CommonPrefixTest2 = {
    val testTree: Tree = {
      orphans: Set(),
      nodes: Map(
        // root node without hash pointing to left internal node and right leaf node
        makeNodeId(0, []) -> Internal({
          left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
          right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
        }),
        makeNodeId(0, [0]) -> Internal({
          left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
          right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
        }),
        makeNodeId(0, [1]) -> Leaf({value_hash: [0], key_hash:[0]}),
          makeNodeId(0, [0,0]) -> Internal({
            left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
            right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
          }),
            makeNodeId(0, [0,0,0]) -> Leaf({value_hash: [0], key_hash:[0,0,0,0]}),
            makeNodeId(0, [0,0,1]) -> Leaf({value_hash: [0], key_hash:[0,0,1,0]}),
          makeNodeId(0, [0,1]) -> Internal({
            left_child: Some({ version: 0, hash : Map([0] -> Raw([0])) }),
            right_child: Some({ version: 0, hash :  Map([0] -> Raw([1])) })
          }),
            makeNodeId(0, [0,1,0]) -> Leaf({value_hash: [0], key_hash:[0,1,0,0]}),
            makeNodeId(0, [0,1,1]) -> Leaf({value_hash: [0], key_hash:[0,1,1,0]})
        )}
    val keyValue = {key_hash : [0,1,1,0], value_hash:[1]}
    val longestPrefixNode = getKeyWithLongestOverlappingPrefix(testTree, keyValue)
    assert({key_hash : [0,1,1], version:0} == longestPrefixNode)
  }

  val testTree: Tree = {
    orphans: Set(),
    nodes: Map(
      // root node without hash pointing to left internal node and right leaf node
      makeNodeId(0, []) -> Internal({
        left_child: Some({ version: 0, hash : hashInternalNode({
          left_child: Some({ version: 0, hash : hashInternalNode({
            left_child: None,
            right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,0,1,0]}) })
          })
        }),
        right_child: Some({ version: 0, hash : hashInternalNode({
            left_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,0,0]}) }),
            right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,1,0]}) })
          })})
        })}),
        right_child: Some({ version: 0, hash :  hashLeafNode({value_hash: [0], key_hash:[0]}) })
      }),
      makeNodeId(0, [0]) -> Internal({
        left_child: Some({ version: 0, hash : hashInternalNode({
          left_child: None,
          right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,0,1,0]}) })
        })}),
        right_child: Some({ version: 0, hash : hashInternalNode({
          left_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,0,0]}) }),
          right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,1,0]}) })
        })})
      }),
        makeNodeId(0, [0,0]) -> Internal({
          left_child: None,
          right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,0,1,0]}) })
        }),
          makeNodeId(0, [0,0,1]) -> Leaf({value_hash: [0], key_hash:[0,0,1,0]}),
        makeNodeId(0, [0,1]) -> Internal({
          left_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,0,0]}) }),
          right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,1,0]}) })
        }),
          makeNodeId(0, [0,1,0]) -> Leaf({value_hash: [0], key_hash:[0,1,0,0]}),
          makeNodeId(0, [0,1,1]) -> Leaf({value_hash: [0], key_hash:[0,1,1,0]}),
      makeNodeId(0, [1]) -> Leaf({value_hash: [0], key_hash:[0]})
    )}

  run InsertTest1 = {
  // sketch of tree used in test
  // state before
  //             root - []
  //            ┌──┴──┐
  //           (0)    1
  //     ┌──────┴──────┐
  //   (00)           (01)
  //  ┌──┴──┐        ┌──┴──┐
  // null  0010     0100  0110

  // newLeaf = {value_hash: [0], key_hash:[0,0,0,1]}
  // tree.insert(newLeaf)
  // state after
  //             root - []
  //            ┌──┴──┐
  //           (0)    1
  //     ┌──────┴──────┐
  //   (00)           (01)
  //  ┌──┴──┐        ┌──┴──┐
  // 000  0010     0100  0110
      val testTree : Tree = {
          orphans: Set(),nodes: Map(
          //root node without hash pointing to left internal node and right leaf node
          makeNodeId(0, []) -> Internal({
              left_child: Some({ version: 0, hash : hashInternalNode({
              left_child: Some({ version: 0, hash : hashInternalNode({
                  left_child: None,
                  right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,0,1,0]}) })
              })}),
              right_child: Some({ version: 0, hash : hashInternalNode({
                  left_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,0,0]}) }),
                  right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,1,0]}) })
              })})
          })}),
              right_child: Some({ version: 0, hash :  hashLeafNode({value_hash: [0], key_hash:[0]}) })
          }),
          makeNodeId(0, [0]) -> Internal({
              left_child: Some({ version: 0, hash : hashInternalNode({
                  left_child: None,
                  right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,0,1,0]}) })
              })}),
              right_child: Some({ version: 0, hash : hashInternalNode({
                  left_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,0,0]}) }),
                  right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,1,0]}) })
              })})
          }),
              makeNodeId(0, [0,0]) -> Internal({
                  left_child: None,
                  right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,0,1,0]}) })
              }),
                  makeNodeId(0, [0,0,1]) -> Leaf({value_hash: [0], key_hash:[0,0,1,0]}),
              makeNodeId(0, [0,1]) -> Internal({
                  left_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,0,0]}) }),
                  right_child: Some({ version: 0, hash : hashLeafNode({value_hash: [0], key_hash:[0,1,1,0]}) })
              }),
                  makeNodeId(0, [0,1,0]) -> Leaf({value_hash: [0], key_hash:[0,1,0,0]}),
                  makeNodeId(0, [0,1,1]) -> Leaf({value_hash: [0], key_hash:[0,1,1,0]}),
          makeNodeId(0, [1]) -> Leaf({value_hash: [0], key_hash:[0]})
      )}
      val newLeaf = { key_hash: [0,0,0,0], value_hash: [0]}
      val nodesOnPath = getNodesOnPath(testTree, newLeaf.key_hash)
      val nodeId = longestPrefixNode(nodesOnPath)
      val newLeaf1 = { key_hash: [0,0,1,1], value_hash: [0]}
      val nodesOnPath1 = getNodesOnPath(testTree, newLeaf1.key_hash)

      all{
          assert(nodesOnPath1 == Set(makeNodeId(0,[]),makeNodeId(0,[0]),makeNodeId(0,[0,0]),makeNodeId(0,[0,0,1]))),
          assert(nodeId == makeNodeId(0,[0,0]))
      }

  }

  pure val empty_tree = { nodes: Map(), orphans: Set() }

  run singleInsertOnEmptyTreeTest =
    assert(
      apply(empty_tree, 0, 1, Set({ key_hash: [0, 1, 1], op: Insert([1, 1, 1, 1]) }))
      == { nodes: Map({ key_hash: [], version: 0 } -> Leaf({ key_hash: [0, 1, 1], value_hash: [1, 1, 1, 1] })),
           orphans: Set() }
    )


  run twoInsertsOnEmptyTreeTest =
    pure val ops = Set(
      { key_hash: [1, 1, 0, 0], op: Insert([1, 1, 1, 1]) }, // a
      { key_hash: [0, 1, 0, 0], op: Insert([1, 1, 1, 0]) } // r
    )

    pure val expected_left = Leaf({ key_hash: [0, 1, 0, 0], value_hash: [1, 1, 1, 0] })
    pure val expected_right = Leaf({ key_hash: [1, 1, 0, 0], value_hash: [1, 1, 1, 1] })
    pure val expected_root = Internal({
      left_child: Some({ version: 0, hash: expected_left.hash() }),
      right_child: Some({ version: 0, hash: expected_right.hash() })
    })

    // TODO: unify with apply_fancy testing once versions are working
    assert(
      apply(empty_tree, 0, 1, ops)
      == { nodes: Map(
           { key_hash: [], version: 0 } -> expected_root,
           { key_hash: [0], version: 0 } -> expected_left,
           { key_hash: [1], version: 0 } -> expected_right
         ), orphans: Set() }
    )

  run twoInsertsWihSharedPrefixOnEmptyTreeTest =
    pure val ops = Set(
      { key_hash: [0, 1, 1, 0], op: Insert([1, 1, 1, 1]) }, // m
      { key_hash: [0, 1, 1, 1], op: Insert([1, 1, 1, 0]) }  // L
    )

    pure val expected_leaf_1 = Leaf({ key_hash: [0, 1, 1, 0], value_hash: [1, 1, 1, 1] })
    pure val expected_leaf_2 = Leaf({ key_hash: [0, 1, 1, 1], value_hash: [1, 1, 1 ,0] })
    pure val internal_height_3 = Internal({
      left_child: Some({ version: 0, hash: expected_leaf_1.hash() }),
      right_child: Some({ version: 0, hash: expected_leaf_2.hash() })
    })
    pure val internal_height_2 = Internal({
      left_child: None,
      right_child: Some({ version: 0, hash: internal_height_3.hash() })
    })
    pure val internal_height_1 = Internal({
      left_child: None,
      right_child: Some({ version: 0, hash: internal_height_2.hash() })
    })
    pure val expected_root = Internal({
      left_child: Some({ version: 0, hash: internal_height_1.hash() }),
      right_child: None
    })

    // TODO: unify with apply_fancy testing once versions are working
    assert(
      apply(empty_tree, 0, 1, ops)
      == { nodes: Map(
           { key_hash: [], version: 0 } -> expected_root,
           { key_hash: [0], version: 0 } -> internal_height_1,
           { key_hash: [0, 1], version: 0 } -> internal_height_2,
           { key_hash: [0, 1, 1], version: 0 } -> internal_height_3,
           { key_hash: [0, 1, 1, 0], version: 0 } -> expected_leaf_1,
           { key_hash: [0, 1, 1, 1], version: 0 } -> expected_leaf_2
         ), orphans: Set() }
    )

  // simple state machine for tests
  var tree: Tree
  action init =
    tree' = {
        orphans: Set(),
        nodes: Map()
  }
  action insertAction(l) =
    pure val leaf_as_op = { key_hash: l.key_hash, op: Insert(l.value_hash) }
    tree' = apply(tree, 0, 0, Set(leaf_as_op))

  run Example = {
    val leafs = List({key_hash: [1, 1, 1, 1], value_hash: [5]}, {key_hash: [0, 0, 0, 0], value_hash: [7]}, {key_hash: [1, 0, 0, 1], value_hash: [42]})
     // TODO non-deterministic initialization
     init
     .then(leafs.length().reps(i => insertAction(leafs[i])))
     .expect(true) //some invariant
  }

  run ExampleSC = {
    val leafs = List({key_hash: [1, 1, 1, 0], value_hash: [5]}, {key_hash: [0, 0, 0, 0], value_hash: [7]}, {key_hash: [1, 1, 0, 1], value_hash: [42]},{key_hash: [1, 1, 1, 1], value_hash: [60]})
    // TODO non-deterministic initialization
    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }

  run ExampleSC1 = {
    val leafs = List({key_hash: [1, 1, 0, 1], value_hash: [5]}, {key_hash: [0, 0, 0, 0], value_hash: [7]}, {key_hash: [1, 1, 1, 0], value_hash: [42]},{key_hash: [1, 1, 1, 1], value_hash: [666]},
    {key_hash: [1, 0, 0, 1], value_hash: [333]})
    // TODO non-deterministic initialization
    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }

  run ExampleSC2 = {
    val leafs = List({key_hash: [1, 1, 0, 1], value_hash: [5]}, {key_hash: [1, 1, 1, 0], value_hash: [42]})
    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }

  run ExampleRustTree = {
    val leafs = List(
      {key_hash: [0, 1, 0, 0], value_hash: [5]},
      {key_hash: [0, 1, 1, 0], value_hash: [33]},
      {key_hash: [0, 1, 1, 1], value_hash: [666]},
      {key_hash: [1, 1, 0, 0], value_hash: [77]})

    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }

  run ExampleRun = {
    val leafs = List(
      { key_hash: [1, 1, 1, 1], value_hash: [5] },
      { key_hash: [1, 1, 1, 0], value_hash: [42] })

    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }
  //      original tree
  //        root
  //        / \
  //       0   1
  //
  //    tree after inserts
  //        root
  //        / \
  //       0   1
  //          / \
  //      null   11
  //            /  \
  //         null  111
  //               /  \
  //            1110  1111
  run ExampleRun2 = {
    val leafs = List(
      { key_hash: [0, 0, 0, 0], value_hash: [33] },
      { key_hash: [1, 1, 1, 1], value_hash: [5] },
      { key_hash: [1, 1, 1, 0], value_hash: [42] })

    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }
}
