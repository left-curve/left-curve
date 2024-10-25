// -*- mode: Bluespec; -*-

module node {
    import hashes.* from "./hashes"
    import basicSpells.* from "./spells/basicSpells"

type Child = {
    version: int, //u64 //TODO: Version not hashed
    hash: Term_t,
}

type InternalNode = {
    left_child: Option[Child],
    right_child: Option[Child],
}

type LeafNode = {
        // In the implementation it is a hash of a key but in the
        // Radix tree it is just used as a key, so we use a list
        // of bit and
        // we treat it here just as bytes    
    key_hash: Bytes_t, 
    value_hash: Bytes_t,
}

type Node = 
    | Internal(InternalNode)
    | Leaf(LeafNode)

pure def is_leaf(n: Node) : bool =
    match n {
        | Leaf(n) => true
        | Internal(n) => false
}
val InternalNodeIdentifier : Bytes_t = [0]
val LeafNodeIdentifier : Bytes_t = [1]
val InternalNodeHashPrefix : Term_t = raw(InternalNodeIdentifier)
val LeafNodeHashPrefix : Term_t = raw(LeafNodeIdentifier)

val l = {
    key_hash: termHash(raw([2])),
    value_hash:  termHash(raw([17]))
}
pure def hashLeafNode(l: LeafNode): Term_t =
    termHash(   LeafNodeHashPrefix
                .termConcat(raw(l.key_hash))
                .termConcat(raw(l.value_hash)))

pure def hashOfChild(oc: Option[Child]) : Term_t =
    match oc {
        | None => termHash(raw([0]))
        | Some(c) => c.hash
    }

pure def hashInternalNode(l: InternalNode): Term_t =
    termHash(   InternalNodeHashPrefix
                .termConcat(l.left_child.hashOfChild())
                .termConcat(l.right_child.hashOfChild()))

pure def hash(n: Node) : Term_t = {
    match n {
        | Leaf(n) => hashLeafNode(n)
        | Internal(n) => hashInternalNode(n)
    }
}
pure def getLeafOrEmpty(n: Node) : LeafNode ={
    match n {
        | Leaf(l) => l
        | Internal(internal) => {key_hash: [], value_hash: []}
    }
}
}