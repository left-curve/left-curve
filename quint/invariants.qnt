// -*- mode: Bluespec; -*-

module invariants {

import tree.* from "./tree" 
import basicSpells.* from "./spells/basicSpells"

/// Make sure the tree encoded in the map forms a tree (everyone has a parent)
/// E.g., if a node has path [1,0,0,1] then there must be a node with path [1,0,0]
pure def everyNodesParentIsInTheTree(t: Tree): bool =
    val paths = t.nodes.keys().map(nId => nId.key_hash)
    paths.forall(p => p.length() > 1 implies paths.contains(p.slice(0, p.length()-1))) 

/// For the same path there are not multiple version stored within the nodes
pure def oneNodePerPath(t: Tree): bool =
    val paths = t.nodes.keys().map(nId => nId.key_hash)
    paths.forall(p => t.nodes.keys().filter(nId => nId.key_hash == p).size() == 1)

pure def interNalNodeHasChildren(n: InternalNode): bool =
    n.left_child != None or n.right_child != None

/// There is no internal nodes without children
pure def allInternalNodesHaveChildren(t: Tree): bool =
    t.nodes.keys().forall(nId => match t.nodes.get(nId) {
        | Internal(n) => interNalNodeHasChildren(n)
        | Leaf(_) => true
    })

pure def getNodeFromPath(t: Tree, b: BitArray): Node =
    val key = t.nodes.keys().filter(nId => nId.key_hash == b).fold({version: 0, key_hash: []}, (s, x) => x)
    t.nodes.get(key)

/// If a node has one child, the child is an internal node
pure def density(t: Tree): bool =
    t.nodes.keys().forall(nId => match t.nodes.get(nId) {
        | Internal(n) => 
                        if (interNalNodeHasChildren(n))
                            if (n.left_child == None)
                                match n.right_child {
                                    | None => true // not reachable
                                    | Some(c) => 
                                        val node = getNodeFromPath(t, nId.key_hash.append(1)) // 1 goes to the right
                                        match node {
                                            | Internal(_) => true
                                            | Leaf(_) => false
                                        }
                                }
                            else if (n.right_child == None)
                                match n.left_child {
                                    | None => true // not reachable
                                    | Some(c) => 
                                        val node = getNodeFromPath(t, nId.key_hash.append(0)) // 0 goes to the left
                                        match node {
                                            | Internal(_) => true
                                            | Leaf(_) => false
                                        }
                                }
                            else
                                false // dead code
                        else true
        | Leaf(_) => true
    })    



}
