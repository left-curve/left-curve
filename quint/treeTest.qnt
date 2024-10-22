// -*- mode: Bluespec; -*-

module treeTest{
// import basicSpells.* from "./basicSpells"
import tree.* from "./tree"   
import node.* from "./node"
import basicSpells.* from "./spells/basicSpells"

run MarkAsOrphanTest = {
//TODO: update testTree to match real tree more closely

    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 5]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
        makeNodeId(2, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 5]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(50, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(61, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(100, [1, 2, 3, 4])-> Internal({left_child: None, right_child: None})
    )
    val testTree = {
        nodes: testNodes,
        orphans: testOrphans
    }
    val o = makeOrphanId(23, 2, [1,2,34,5,6])

    val updateTree = mark_node_as_orphaned(testTree, o.orphaned_since_version, o.version, o.key_hash)
    assert(updateTree.orphans ==  Set( 
            makeOrphanId(22, 2, [1, 2, 3, 4]),
            makeOrphanId(232, 3, [1, 2, 3, 4]),
            makeOrphanId(231, 3, [1, 2, 3, 5]),
            makeOrphanId(199, 50, [1, 2, 3, 4]),
            makeOrphanId(11, 61, [1, 2, 3, 4]),
            makeOrphanId(2322, 100, [1, 2, 3, 4]),
            makeOrphanId(23, 2, [1,2,34,5,6])    
        ))
}

run Prune1Test = {
//TODO: update testTree to match real tree more closely

    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 5]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )    
    val testNodes = Map(
        makeNodeId(2, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 5]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(50, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(61, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(100, [1, 2, 3, 4])-> Internal({left_child: None, right_child: None})
    )
    val testTree = {
        nodes: testNodes,
        orphans: testOrphans
    }
    val ret = prune(testTree, 200)
    all{
        assert(ret.orphans == Set( 
            makeOrphanId(232, 3, [1, 2, 3, 4]),
            makeOrphanId(231, 3, [1, 2, 3, 5]),
            makeOrphanId(2322, 100, [1, 2, 3, 4])
        )),
        assert(ret.nodes == Map( 
            makeNodeId(3, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
            makeNodeId(3, [1, 2, 3, 5]) -> Internal({left_child: None, right_child: None}),
            makeNodeId(100, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None})
        )),
    }
    
}
run Prune2Test = {
//TODO: update testTree to match real tree more closely

    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 5]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
        makeNodeId(2, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 5]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(50, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(61, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(100, [1, 2, 3, 4])-> Internal({left_child: None, right_child: None})
    )    
    val testTree = {
        nodes: testNodes,
        orphans: testOrphans
    }
    val ret = prune(testTree, 2500)
    all {
        assert(ret.nodes == Map()),
        assert(ret.orphans == Set())
    }
}
run Prune3Test = {
//TODO: update testTree to match real tree more closely

    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 5]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
        makeNodeId(2, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(3, [1, 2, 3, 5]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(50, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(61, [1, 2, 3, 4]) -> Internal({left_child: None, right_child: None}),
        makeNodeId(100, [1, 2, 3, 4])-> Internal({left_child: None, right_child: None})
    )    

    val testTree = {
        nodes: testNodes,
        orphans: testOrphans
    }
    val ret = prune(testTree, 10)
    all {
        assert(ret.orphans == testOrphans),
        assert(ret.nodes == testNodes)
    }
    
}
run PruneFunctionTests = {
    Prune1Test.then(Prune2Test).then(Prune3Test)
}

run CommonPrefixTest1 = {
    val testTree : Tree = { 
        orphans: Set(),nodes: Map(
        //root node without hash pointing to left internal node and right leaf node
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
    val testTree : Tree = { 
        orphans: Set(),nodes: Map(
        //root node without hash pointing to left internal node and right leaf node
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
val newLeaf = {value_hash: [0], key_hash:[0,0,1,1]}
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
val t = {orphans: Set(),
        nodes: Map()}
// type Tree = int -> int //to do

// type Operation

// pure def apply(t: Tree, op: Operation) : Tree =
//     t

// pure def applyFancy(t: Tree, l: List[Operation]) : Tree =
//     t

// run treeTest =
//     val l = List() // generate random inputs
//     val tree = Map()
//     val reference = l.foldl(tree, (s,x) => apply(s,x))
//     val result = applyFancy(tree, l)
//     assert(reference == result)

}