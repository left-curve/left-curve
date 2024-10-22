// -*- mode: Bluespec; -*-

module invariants {

import tree.* from "./tree" 
import basicSpells.* from "./spells/basicSpells"

val keylength = 4

/// Make sure the tree encoded in the map forms a tree (everyone has a parent)
/// E.g., if a node has path [1,0,0,1] then there must be a node with path [1,0,0]
// TODO: I think this is wrong. The point is the first node that is added doesn't have any
// inner node parent. Inner nodes are generated from common prefixes
pure def everyNodesParentIsInTheTree(t: Tree): bool =
    val paths = t.nodes.keys().map(nId => nId.key_hash)
    paths.forall(p => p.length() > 1 implies paths.contains(p.slice(0, p.length()-1))) 

pure def commonPrefix(a: LeafNode, b: LeafNode) : Bytes_t =
    val indList = range(1, keylength + 1)
    indList.foldl([], (s, x) => if (a.key_hash.slice(0, x) == b.key_hash.slice(0, x)) 
                                    b.key_hash.slice(0, x) 
                                else s )

pure def listCommonPrefix(a: BitArray, b: BitArray) : BitArray =
    []

pure def longestPrefixNode(s: Set[NodeId]) : NodeId =
    {version: 0, key_hash: []}

pure def newInternal(a: LeafNode, b: LeafNode): InternalNode =
    // TODO: hashes, etc.
    {left_child: None, right_child: None}

pure def addLeafUnderInternal(i: InternalNode, l: LeafNode): InternalNode =
    // TODO: hashes, etc.
    // figure out whether l is left or right, and puts the hash(l) there
        i
pure def newPrefix(l: LeafNode, parent: NodeId): NodeId = 
    // prefix of leafNode key of lenght parent.key.length + 1 
    // and proper version
    parent

// forget empty tree
pure def insert(t: Tree, l: LeafNode): Tree = 
    val nodeIdOnPath = t.nodes.keys().filter(prefix => listCommonPrefix(prefix.key_hash,l.key_hash).length() > 0)
    val nodeId: NodeId = longestPrefixNode(nodeIdOnPath)
    val nodeToChange = t.nodes.get(nodeId)
    val replacement: InternalNode = match nodeToChange {
        | Leaf(leaf) => newInternal(leaf, l)      
        // 0011 -> LeafA
        // Add leafB
        // 0011 -> Internal
        // 00111 -> A (or B)
        // 00110 -> B (or A)
        | Internal(internal) => addLeafUnderInternal(internal, l)
        // 0011 -> Internal
        // 00111 -> Leaf A
        // 00110000 -> l
        // Then
        // 0011 -> replacement
        // newPrefix(l, 0011) = 00110
        // add 00110 -> l
    }
    // TODO: In Leaf case above, we need to
    val newNodes = t.nodes
                        .put(nodeId, Internal(replacement))
                        .set(newPrefix(l,nodeId), Leaf(l))        
    val newnew = match   nodeToChange {
        | Leaf(leaf) => newNodes.set(newPrefix(leaf,nodeId), Leaf(leaf))  
        | Internal(internal) => newNodes      
    }   
    // TODO: use nodeIdOnPath to iterate in descending length()
    t.with("nodes", newNodes)
    // TODO: add special cases of empty tree, and only one leaf
    // TODO: delete function + update
    // TODO: implement helper functions


pure def allLeafs(t: Tree) : Set[LeafNode] =
    val allNodes = t.nodes.keys().fold(Set(), (s, x) => s.union(Set(t.nodes.get(x))))
    allNodes.fold(Set(), (s,x) => match x {
                            | Internal(_) => s
                            | Leaf(x) => s.union(Set(x))
    })

pure def existsNode(t: Tree, b: Bytes_t): bool =
    t.nodes.keys().filter(nId => nId.key_hash == b).size() > 0

pure def nodeAtCommonPrefix(t: Tree) : bool =
    t.allLeafs().forall(a =>
        t.allLeafs().forall(b =>
            (a.key_hash != b.key_hash) implies existsNode(t, commonPrefix(a,b))   
    ))


// TODO NodeId -> Node is a bijection



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
