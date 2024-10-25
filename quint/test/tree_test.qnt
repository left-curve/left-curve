// -*- mode: Bluespec; -*-

module tree_test {
  import tree.* from "../tree"
  import node.* from "../node"
  import basicSpells.* from "../spells/basicSpells"

  pure def makeOrphanId(osv: Version, v: Version, kh: List[int]): OrphanId =
    { orphaned_since_version: osv, version: v, key_hash: kh }

  pure def makeNodeId(v: Version, kh: BitArray): NodeId =
    { version: v, key_hash: kh }

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
