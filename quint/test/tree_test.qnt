// -*- mode: Bluespec; -*-

module tree_test {
  import tree.* from "../tree"
  import node.* from "../node"
  import utils.* from "../utils"
  import basicSpells.* from "../spells/basicSpells"
  import apply_state_machine.* from "../apply_state_machine"
  import apply_simple as simple from "../apply_simple"
  import apply_fancy as fancy from "../apply_fancy"
  import apply_super_simple as super_simple from "../apply_super_simple"

  pure def makeOrphanId(osv: Version, v: Version, kh: List[int]): OrphanId =
    { orphaned_since_version: osv, version: v, key_hash: kh }

  pure def makeNodeId(v: Version, kh: BitArray): NodeId =
    { version: v, key_hash: kh }

  run MarkAsOrphanTest = {
    // TODO: update testTree to match real tree more closely

    val testOrphans = Set(
      makeOrphanId(22, 2, [1, 2, 3, 4]),
      makeOrphanId(232, 3, [1, 2, 3, 4]),
      makeOrphanId(231, 3, [1, 2, 3, 5]),
      makeOrphanId(199, 50, [1, 2, 3, 4]),
      makeOrphanId(11, 61, [1, 2, 3, 4]),
      makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
      makeNodeId(2, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 5]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(50, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(61, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(100, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None })
    )
    val testTree = {
      nodes: testNodes,
      orphans: testOrphans
    }
    val o = makeOrphanId(23, 2, [1, 2, 34, 5, 6])

    val updateTree = mark_node_as_orphaned(testTree, o.orphaned_since_version, o.version, o.key_hash)
    assert(updateTree.orphans == Set(
      makeOrphanId(22, 2, [1, 2, 3, 4]),
      makeOrphanId(232, 3, [1, 2, 3, 4]),
      makeOrphanId(231, 3, [1, 2, 3, 5]),
      makeOrphanId(199, 50, [1, 2, 3, 4]),
      makeOrphanId(11, 61, [1, 2, 3, 4]),
      makeOrphanId(2322, 100, [1, 2, 3, 4]),
      makeOrphanId(23, 2, [1, 2, 34, 5, 6])
    ))
  }

  run Prune1Test = {
    // TODO: update testTree to match real tree more closely

    val testOrphans = Set(
      makeOrphanId(22, 2, [1, 2, 3, 4]),
      makeOrphanId(232, 3, [1, 2, 3, 4]),
      makeOrphanId(231, 3, [1, 2, 3, 5]),
      makeOrphanId(199, 50, [1, 2, 3, 4]),
      makeOrphanId(11, 61, [1, 2, 3, 4]),
      makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
      makeNodeId(2, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 5]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(50, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(61, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(100, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None })
    )
    val testTree = {
      nodes: testNodes,
      orphans: testOrphans
    }
    val ret = prune(testTree, 200)
    all {
      assert(ret.orphans == Set(
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 5]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
      )),
      assert(ret.nodes == Map(
        makeNodeId(3, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
        makeNodeId(3, [1, 2, 3, 5]) -> Internal({ left_child: None, right_child: None }),
        makeNodeId(100, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None })
      )),
    }
  }

  run Prune2Test = {
    // TODO: update testTree to match real tree more closely

    val testOrphans = Set(
      makeOrphanId(22, 2, [1, 2, 3, 4]),
      makeOrphanId(232, 3, [1, 2, 3, 4]),
      makeOrphanId(231, 3, [1, 2, 3, 5]),
      makeOrphanId(199, 50, [1, 2, 3, 4]),
      makeOrphanId(11, 61, [1, 2, 3, 4]),
      makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
      makeNodeId(2, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 5]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(50, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(61, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(100, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None })
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
    // TODO: update testTree to match real tree more closely

    val testOrphans = Set(
      makeOrphanId(22, 2, [1, 2, 3, 4]),
      makeOrphanId(232, 3, [1, 2, 3, 4]),
      makeOrphanId(231, 3, [1, 2, 3, 5]),
      makeOrphanId(199, 50, [1, 2, 3, 4]),
      makeOrphanId(11, 61, [1, 2, 3, 4]),
      makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val testNodes = Map(
      makeNodeId(2, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(3, [1, 2, 3, 5]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(50, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(61, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None }),
      makeNodeId(100, [1, 2, 3, 4]) -> Internal({ left_child: None, right_child: None })
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

  // FIXME: this test fails and is skipped
  run simpleVsFancyTestSkipped = {
    pure val empty_tree = { nodes: Map(), orphans: Set() }
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()
    pure val ops = kms_with_value.to_operations()

    pure val reference = simple::apply(empty_tree, 0, 0, ops)
    pure val result = fancy::apply(empty_tree, 0, 0, ops)
    assert(reference == result)
  }

  run superSimpleVsFancyTest = init.then(3.reps(_ => {
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()
    pure val ops = kms_with_value.to_operations()

    val reference = super_simple::apply(tree, version - 1, version, ops)
    val result = fancy::apply(tree, version - 1, version, ops)
    all {
      tree' = result,
      version' = version + 1,
      ops_history' = q::debug("ops_history", ops_history.append(ops)),
      smallest_unpruned_version' = smallest_unpruned_version,
      if (reference != result) assert(q::debug("super_simple", reference) == q::debug("fancy", result)) else true,
    }
  }))

  // FIXME: this test fails and is skipped
  run superSimpleVsSimpleTestSkipped = {
    pure val empty_tree = { nodes: Map(), orphans: Set() }
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()
    pure val ops = kms_with_value.to_operations()

    pure val reference = super_simple::apply(empty_tree, 0, 0, ops)
    pure val result = simple::apply(empty_tree, 0, 0, ops)
    assert(reference == result)
  }
}
