// -*- mode: Bluespec; -*-

module tree {

import hashes.* from "./hashes"
import basicSpells.* from "./spells/basicSpells"

type Index = int
val IndexRange = 0.to(15)

pure def indicesWithinRange(s: Set[Index]) : bool =
    s.forall(i => IndexRange.contains(i))

// TODO: Question: what is special about the default digest.
val defaultDigest = [0]

type Digest = Term_t 

type NibblePath = List[Index] 

type InnerNodeData = {
    version: int,
    digest: Digest,
}

type NodeKey = {
    version: int,
    path: NibblePath // TODO: from paper not clear whether this is all, or whether there is a suffix with the remainder
}

type InnerNode = {
    key: NodeKey,
    subtree: Index -> InnerNodeData,
}

type LeafNode = {
    key: NodeKey,
    value_hash: Digest,
}

type Node = 
    | Inner(InnerNode)
    | Leaf(LeafNode)

type Tree = {
    rootHash: Digest,
    nodes: Set[Node],
}


pure def insert (t: Tree, data: Bytes_t, key: NibblePath, version:int): Tree =
// TODO: Insert data into tree
// TODO: check whether there is a difference to update (the paper says insertion and update)
    t

pure def lookup (t: Tree, key: NibblePath, version:int) : Option[Bytes_t] = 
// TODO: check if data is there and return it otherwise None
    None

// TODO: big todo: figure out whether there is a tree invariant.
// TODO: do some random simulation of insert to generate trees
//       simulate insertion and check invariant
// TODO: we should write constraints to generate trees at the initial state

// TODO: encode inclusion proofs and non-inclusion proofs
// TODO: encode verifyMembership and verifyNonMembership

var tree: Tree

action init =
    tree' = {
        rootHash: Map([0] -> Raw(defaultDigest)),
        nodes: Set(),
    }


// TODO: ask about versions

val fig3 = {
    rootHash: Map([0] -> Raw(defaultDigest)),
    nodes: Set(
        Inner({key: {version:0, path:[13]}, subtree: Map(12 -> {version:0, digest: Map([0] -> Raw([13]))})}),
        Inner({key: {version:0, path:[1]}, subtree: Map()}),        
        Leaf({key: {version:0, path:[13, 12]}, value_hash: Map([0] -> Raw([13]))}),
        Leaf({key: {version:1, path:[13, 12]}, value_hash: Map([0] -> Raw([14]))})
    )

}

}