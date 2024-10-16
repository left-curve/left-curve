// -*- mode: Bluespec; -*-

module node {
    import hashes.* from "./hashes"

type Option[a] = 
    | None
    | Some(a)

type Child = {
    version: int, //u64 //TODO: Version not hashed
    hash: Term_t,
}

type InternalNode = {
    left_child: Option[Child],
    right_child: Option[Child],
}

type LeafNode = {
    key_hash: Term_t,
    value_hash: Term_t,
}

type Node = 
    | Internal(InternalNode)
    | Leaf(LeafNode)

val InternalIdentifier: Bytes_t = [0]
val LeafNodeIdentifier: Bytes_t = [1]
val InternalNodeHashPrefix: Term_t = raw(InternalIdentifier)
val LeafNodeHashPrefix: Term_t = raw(LeafNodeIdentifier)

val l = {
    key_hash: termHash(raw([2])),
    value_hash:  termHash(raw([17]))
}

pure def hashLeafNode(l: LeafNode): Term_t =
    termHash(   LeafNodeHashPrefix
                .termConcat(l.key_hash)
                .termConcat(l.value_hash))

run LeafHashTest = {  
    nondet key = 0.to(16).oneOf()
    nondet value = 0.to(16).oneOf()
    val l = {
        key_hash: termHash(raw([key])),
        value_hash:  termHash(raw([value]))
    }
    val result = hashLeafNode(l)
    val expected =  Map([0] -> Hash,
                            [0, 0] -> Raw(LeafNodeIdentifier),
                            [0, 1] -> Hash, 
                                [0, 1, 0] -> Raw([key]), 
                            [0, 2] -> Hash,        
                                [0, 2, 0] -> Raw([value]) 
                )
    assert(result == expected)
}


pure def hashOfChild(oc: Option[Child]) : Term_t =
    match oc {
        | None => termHash(raw([0]))
        | Some(c) => c.hash
    }

pure def hashInternalNode(l: InternalNode): Term_t =
    termHash(   InternalNodeHashPrefix
                .termConcat(l.left_child.hashOfChild())
                .termConcat(l.right_child.hashOfChild()))


// TODO test

}