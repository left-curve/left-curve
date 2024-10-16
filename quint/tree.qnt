module tree{

import basicSpells.* from "./spells/basicSpells"
import rareSpells.* from "./spells/rareSpells"
import node.* from "./node"

type Outcome = 
    | Unchanged(Option[Node])
    | Updated(Node)
    | Deleted

type BitArray = List[int]
type Version = int 

type Tree = {
    nodes : (Version, BitArray) -> Node,
    orphans: Set[(Version, Version, BitArray)]
}

pure def insert_node(n: Node) : bool = {
    true
}
pure def delete_node(n: Node) : bool = {
    true
}
pure def update_node(n: Node) : bool = {
    true
}
pure def mark_node_as_orphaned(orphans: Set[(Version, Version, BitArray)], orphaned_since_version: Version, version: Version, bits: BitArray) : Set[(Version, Version, BitArray)] = {
    orphans.union(Set((orphaned_since_version, version, bits)))
}
pure def prune(orphans: Set[(Version, Version, BitArray)], up_to_version: Version) : Set[(Version, Version, BitArray)] =  {
    orphans.filter(orphan => orphan._1 > up_to_version)
}

var tree : Tree

action init =
    tree' = {
        orphans: Set(),
        nodes: Map(),
    }





// -------------------- tests --------------------

run MarkAsOrphan1Test = {
    val testOrphans = Set( (22, 2, [1, 2, 3, 4]), (232, 3, [1, 2, 3, 4]), (231, 3, [1, 2, 3, 4]), (199, 50, [1, 2, 3, 4]), (11, 61, [1, 2, 3, 4]), (2322, 100, [1, 2, 3, 4]))
    val newOrphan_version = 2
    val newOrphan_since_version = 23
    val newOrphan_bits = [1,2,34,5,6]
    val updatedOrphans = mark_node_as_orphaned(testOrphans, newOrphan_since_version, newOrphan_version, newOrphan_bits)
    assert(updatedOrphans == Set( (22, 2, [1, 2, 3, 4]), (232, 3, [1, 2, 3, 4]), (231, 3, [1, 2, 3, 4]), (199, 50, [1, 2, 3, 4]), (11, 61, [1, 2, 3, 4]), (2322, 100, [1, 2, 3, 4]), (23, 2, [1,2,34,5,6])))
}

run Prune1Test = {
    val testOrphans = Set( (22, 2, [1, 2, 3, 4]), (232, 3, [1, 2, 3, 4]), (231, 3, [1, 2, 3, 4]), (199, 50, [1, 2, 3, 4]), (11, 61, [1, 2, 3, 4]), (2322, 100, [1, 2, 3, 4]))
    val ret = prune(testOrphans, 200)

    assert(ret == Set( (232, 3, [1, 2, 3, 4]), (231, 3, [1, 2, 3, 4]), (2322, 100, [1, 2, 3, 4])))
}
run Prune2Test = {
    val testOrphans = Set( (22, 2, [1, 2, 3, 4]), (232, 3, [1, 2, 3, 4]), (231, 3, [1, 2, 3, 4]), (199, 50, [1, 2, 3, 4]), (11, 61, [1, 2, 3, 4]), (2322, 100, [1, 2, 3, 4]))
    val ret = prune(testOrphans, 2500)
    assert(ret == Set())
}
run Prune3Test = {
    val testOrphans = Set( (22, 2, [1, 2, 3, 4]), (232, 3, [1, 2, 3, 4]), (231, 3, [1, 2, 3, 4]), (199, 50, [1, 2, 3, 4]), (11, 61, [1, 2, 3, 4]), (2322, 100, [1, 2, 3, 4]))
    val ret = prune(testOrphans, 10)
    assert(ret == testOrphans)
}

run testPruneFunction = {
    Prune1Test.then(Prune2Test).then(Prune3Test)
}

}