module treeTest{

import tree.* from "./tree"    
run MarkAsOrphanTest = {
    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 4]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val o = makeOrphanId(23, 2, [1,2,34,5,6])

    val updatedOrphans = mark_node_as_orphaned(testOrphans, o)
    assert(updatedOrphans ==  Set( 
            makeOrphanId(22, 2, [1, 2, 3, 4]),
            makeOrphanId(232, 3, [1, 2, 3, 4]),
            makeOrphanId(231, 3, [1, 2, 3, 4]),
            makeOrphanId(199, 50, [1, 2, 3, 4]),
            makeOrphanId(11, 61, [1, 2, 3, 4]),
            makeOrphanId(2322, 100, [1, 2, 3, 4]),
            makeOrphanId(23, 2, [1,2,34,5,6])    
        ))
}

run Prune1Test = {
    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 4]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val ret = prune(testOrphans, 200)
    assert(ret == Set( 
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    ))
}
run Prune2Test = {
    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 4]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val ret = prune(testOrphans, 2500)
    assert(ret == Set())
}
run Prune3Test = {
    val testOrphans = Set( 
        makeOrphanId(22, 2, [1, 2, 3, 4]),
        makeOrphanId(232, 3, [1, 2, 3, 4]),
        makeOrphanId(231, 3, [1, 2, 3, 4]),
        makeOrphanId(199, 50, [1, 2, 3, 4]),
        makeOrphanId(11, 61, [1, 2, 3, 4]),
        makeOrphanId(2322, 100, [1, 2, 3, 4])
    )
    val ret = prune(testOrphans, 10)
    assert(ret == testOrphans)
}
run PruneFunctionTests = {
    Prune1Test.then(Prune2Test).then(Prune3Test)
}

}