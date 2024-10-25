// -*- mode: Bluespec; -*-

module invariants {


import tree.* from "./tree"
import apply_state_machine.* from "./apply_state_machine"
import basicSpells.* from "./spells/basicSpells"
import utils.* from "./utils"

type TreeMap = NodeId -> Node

val maxVersion = 10

/// return the prefixes of all childred from nodes at the given version 
pure def ChildrenPrefixes(nodes: TreeMap, version: int): Set[BitArray] = 
  val prefixes = nodes.keys()
                      .filter(nId => nId.version == version)
                      .map(nId => nId.key_hash)
  prefixes.fold(Set(), (s, x) =>
    match nodes.get({version: version, key_hash: x}) {
      | Internal(n) =>
        val lc = if (n.left_child != None) 
                  Set(x.append(0)) 
                else 
                  Set()
        val rc = if (n.right_child != None) 
                  Set(x.append(1)) 
                else 
                  Set()
        s.union(lc).union(rc)  
      | Leaf(_) => s
    }
  )

/// return the NodeId for the given prefix with the largest version less than the given version
pure def getnodeIdforVersion(nodes: TreeMap, key: BitArray, version: int): NodeId = 
  nodes .keys()
        .filter(nId => nId.key_hash == key and nId.version <= version)
        .fold({ version: -1, key_hash: key }, (s,x) => 
          if (x.version > s.version)
            x
          else 
          s
        )

/// compute the tree at the given version
pure def treeAtVersion(t: Tree, version: int): TreeMap =
  val nodesUpToVersion = t.nodes.keys()
                                .filter(nId => nId.version <= version)
                                .mapBy(nId => t.nodes.get(nId))
  val nodesAtVersion = t.nodes.keys()
                              .filter(nId => nId.version == version)
                              .mapBy(nId => t.nodes.get(nId))
  val indList = range(1, version + 1) // TODO check borders
  // go back in versions and add childred nodes
  indList.foldr(nodesAtVersion, (ver, treeNodes) => 
    // for the current versions check the childred to add
    // add them one-by-one in the fold, but pick the largest version of this child
    ChildrenPrefixes(nodesUpToVersion, ver).fold(treeNodes, (ss, xx) => 
      val nodeId = getnodeIdforVersion(nodesUpToVersion, xx, ver)
      ss.set(nodeId, nodesUpToVersion.get(nodeId)))
  )
    



// TODO: if an internal node has one child, then the child must be internal node

/// Make sure the tree encoded in the map forms a tree (everyone has a parent)
/// E.g., if a node has keyhash_prefix [1,0,0,1] then there must be a node with keyhash_prefix [1,0,0]
// TODO: figure out versions. I guess we need a parent with a version >= my version
pure def everyNodesParentIsInTheTree(t: TreeMap): bool =
  val nodePrefixes = t.keys()
  val paths = nodePrefixes.map(p => p.key_hash)
  paths.forall(p => p.length() > 1 implies paths.contains(p.slice(0, p.length()-1))) 

/// Invariant: Everyone has a parent
/// E.g., if a node has keyhash_prefix [1,0,0,1] then there must be a node with keyhash_prefix [1,0,0]
def everyNodesParentIsInTheTreeInv = 
  0.to(maxVersion).forall(v =>
    val vTree = treeAtVersion(tree,v)
    everyNodesParentIsInTheTree(vTree)
  )

/// Is the node an internal node?
pure def isInternal(n: Node) : bool =
    match n {
        | Internal(_) => true
        | Leaf(_) => false
    }
/// Is the node a leaf node?
pure def isLeaf(n) = (not(isInternal(n)))

// pure def allInternalwithPrefix(t: Tree) : TreeMap =
//     val leafNodeIds = t.nodes.keys().filter(nId => t.nodes.get(nId).isInternal())
//     leafNodeIds.mapBy(p => t.nodes.get(p))

// pure def allLeafswithPrefix(t: Tree) : TreeMap =
//     val internalNodeIds = t.nodes.keys().filter(nId => t.nodes.get(nId).isLeaf())
//     internalNodeIds.mapBy(p => t.nodes.get(p))


/// Check whether there is a node with the given prefix
/// TODO: currently it ignores versions. Instead of Bytes_t it should take a NodeId as input
pure def existsNode(t: TreeMap, b: Bytes_t): bool =
    t.keys().filter(nId => nId.key_hash == b).size() > 0

/// For any two leaf nodes, is there is a nodeId in the tree that is the common prefix
/// of the nodes?
pure def nodeAtCommonPrefix(t: TreeMap) : bool =
    t.allLeafs().forall(a =>
        t.allLeafs().forall(b =>
            (a.key_hash != b.key_hash) implies existsNode(t, commonPrefix(a,b))   
    ))

/// Invariant that checks that for any two leaf nodes, there is a nodeId in the
/// tree that is the common prefix
def nodeAtCommonPrefixInv = 
  0.to(maxVersion).forall(v =>
    val vTree = treeAtVersion(tree,v)
    nodeAtCommonPrefix(vTree)
  )

/// This computes the prefix of the child. 
/// TODO: currently it ignores versions
pure def rightChild(t: TreeMap, nId: NodeId): NodeId =
    // TODO: versions
    {version: 0, key_hash: nId.key_hash.append(1)}

/// This computes the prefix of the child. 
/// TODO: currently it ignores versions
pure def leftChild(t: TreeMap, nId: NodeId): NodeId =
    // TODO: versions
    // there should only be one node for each prefix, but we need to figure out its version
    {version: 0, key_hash: nId.key_hash.append(0)}   


/// Check that for all internal nodes, if they have a hash stored for a child,
/// then the hash is the hash of the actualy the child
pure def properlyHashed(t: TreeMap): bool = 
    t.keys().forall(nID => match t.get(nID) {
                                | Leaf(_) => true
                                | Internal(n) => and {
                                    match n.left_child {
                                        | None => true
                                        | Some(c) => c.hash == hash(t.get(leftChild(t, nID)))
                                    },
                                    match n.right_child {
                                        | None => true
                                        | Some(c) => c.hash == hash(t.get(rightChild(t, nID)))
                                    }
                                }
    })    

/// Invariant: All hashes stored in the tree are actually the hashes of the subtree
def hashInv = 
    0.to(maxVersion).forall(v => properlyHashed(treeAtVersion(tree,v)))

// TODO TreeMap is a bijection: perhaps not with versions

pure def isPrefix(short: BitArray, long: BitArray): bool =
    if (short.length() > long.length())
        false
    else
        short == long.slice(0, short.length() - 1)

/// Make sure that the map encodes a tree. In particular, there is no internal node 
/// that has a leaf node as its prefix
/// E.g., the map is not allowed to have: [0,0,1] -> internal node; [0,0] -> leaf node
/// TODO: versions
pure def noInternalChild(t: Tree): bool =
    val leafNodeIds = t.nodes.keys().filter(nId => t.nodes.get(nId).isInternal())
    val internalNodeIds = t.nodes.keys().filter(nId => t.nodes.get(nId).isLeaf())
    val leafPrefixes = leafNodeIds.map(nId => nId.key_hash)
    val internalPrefixes = internalNodeIds.map(nId => nId.key_hash)
    not(leafPrefixes.exists(lp => 
            internalPrefixes.exists(ip => 
                isPrefix(lp, ip))))

/// Invariant: there is no internal node  that has a leaf node as its prefix
def noInternalChildInv = noInternalChild(tree)

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

/// Invariant: All nodes of type Internal have a child which is not None
def allInternalNodesHaveChildrenInv = allInternalNodesHaveChildren(tree)

pure def getNodeFromPath(t: Tree, b: BitArray): Node =
    val key = t.nodes.keys().filter(nId => nId.key_hash == b).fold({version: 0, key_hash: []}, (s, x) => x)
    t.nodes.get(key)


/// If a node (different from the root) has one child, the child is an internal node
pure def density(t: Tree): bool =
    t.nodes.keys().filter(nId => nId.key_hash != []).forall(nId => match t.nodes.get(nId) {
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
                            else // it has two children
                                true 
                        else true
        | Leaf(_) => true
    })    

def densityInv = density(tree)

}
