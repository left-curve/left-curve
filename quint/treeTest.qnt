// -*- mode: Bluespec; -*-

module treeTest {


type Tree = int -> int //to do

type Operation

pure def apply(t: Tree, op: Operation) : Tree =
    t

pure def applyFancy(t: Tree, l: List[Operation]) : Tree =
    t

run treeTest =
    val l = List() // generate random inputs
    val tree = Map()
    val reference = l.foldl(tree, (s,x) => apply(s,x))
    val result = applyFancy(tree, l)
    assert(reference == result)


}