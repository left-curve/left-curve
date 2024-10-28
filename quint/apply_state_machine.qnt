// -*- mode: Bluespec; -*-

module apply_state_machine {
  import basicSpells.* from "./spells/basicSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import utils.* from "./utils"

  import apply_simple as simple from "./apply_simple"
  import apply_fancy as fancy from "./apply_fancy"

  var tree: Tree
  var version: int
  var pruned_until: int

  action init = all {
    // For now, we always start with an empty tree
    tree' = { nodes: Map(), orphans: Set() },
    version' = 1,
    pruned_until' = 0
  }

  pure val all_key_hashes_as_maps = (0.to(MAX_HASH_LENGTH).setOfMaps(Set(0, 1))).powerset()
  pure def key_hash_map_to_op(km: int -> int): OperationOnKey = {
    pure val key_hash: Bytes_t = range(0, MAX_HASH_LENGTH).foldl([], (acc, i) => acc.append(km.get(i)))
    { key_hash: key_hash, op: Insert(key_hash) } // value_hash is equal to key_hash to make things easier
  }

  action step_fancy = all {
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    pure val ops = key_hashes_as_maps.map(key_hash_map_to_op)
    tree' = fancy::apply(tree, version - 1, version, ops),
    version' = version + 1,
    pruned_until' = pruned_until
  }

  action step_simple = all {
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    pure val ops = key_hashes_as_maps.map(key_hash_map_to_op)
    tree' = simple::apply(tree, version - 1, version, ops),
    version' = version + 1,
    pruned_until' = pruned_until
  }

  /********* INVARIANTS ***********/

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
    pruned_until.to(tree.treeVersion()).forall(v =>
      val vTree = treeAtVersion(tree,v)
      everyNodesParentIsInTheTree(vTree)
    )

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
    pruned_until.to(tree.treeVersion()).forall(v =>
      val vTree = treeAtVersion(tree,v)
      nodeAtCommonPrefix(vTree)
    )

  /// This computes the prefix of the child.
  pure def rightChild(t: TreeMap, nId: NodeId): NodeId =
      // there should only be one node for each prefix, but we need to figure out its version
      t.keys().filter(n => n.key_hash == nId.key_hash.append(1))  // filter down to a singleton
              .fold({ version: -1, key_hash: []}, (s, x) => x)    // pick the one element

  /// This computes the prefix of the child.
  pure def leftChild(t: TreeMap, nId: NodeId): NodeId =
      // there should only be one node for each prefix, but we need to figure out its version
      t.keys().filter(n => n.key_hash == nId.key_hash.append(0))  // filter down to a singleton
              .fold({ version: -1, key_hash: []}, (s, x) => x)    // pick the one element

  /// The treemaps we use in the invariants have unique key_hashes
  pure def goodTreeMap(t: TreeMap): bool =
    t.keys().forall(a =>
      t.keys().forall(b => a.key_hash == b.key_hash implies a.version == b.version))

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
      pruned_until.to(tree.treeVersion()).forall(v => properlyHashed(treeAtVersion(tree,v)))

  // TODO TreeMap is a bijection: perhaps not with versions

  /// Make sure that the map encodes a tree. In particular, there is no internal node
  /// that has a leaf node as its prefix
  /// E.g., the map is not allowed to have: [0,0,1] -> internal node; [0,0] -> leaf node
  /// TODO: versions
  pure def noInternalChild(t: TreeMap): bool =
      val leafNodeIds = t.keys().filter(nId => t.get(nId).isLeaf())
      val internalNodeIds = t.keys().filter(nId => t.get(nId).isInternal())
      val leafPrefixes = leafNodeIds.map(nId => nId.key_hash)
      val internalPrefixes = internalNodeIds.map(nId => nId.key_hash)
      not(leafPrefixes.exists(lp =>
              internalPrefixes.exists(ip =>
                  isPrefixOf(lp, ip))))

  /// Invariant: there is no internal node  that has a leaf node as its prefix
  def noInternalChildInv =
      pruned_until.to(tree.treeVersion()).forall(v => noInternalChild(treeAtVersion(tree,v)))

  /// For the same path there are not multiple version stored within the nodes
  pure def oneNodePerPath(t: Tree): bool =
      val paths = t.nodes.keys().map(nId => nId.key_hash)
      paths.forall(p => t.nodes.keys().filter(nId => nId.key_hash == p).size() == 1)

  pure def internalNodeHasChildren(n: InternalNode): bool =
      n.left_child != None or n.right_child != None

  /// There is no internal nodes without children
  pure def allInternalNodesHaveChildren(t: TreeMap): bool =
      t.keys().forall(nId => match t.get(nId) {
          | Internal(n) => internalNodeHasChildren(n)
          | Leaf(_) => true
      })

  /// Invariant: All nodes of type Internal have a child which is not None
  def allInternalNodesHaveChildrenInv =
    pruned_until.to(tree.treeVersion()).forall(v => allInternalNodesHaveChildren(treeAtVersion(tree,v)))

  pure def getNodeFromPath(t: TreeMap, b: BitArray): Node =
      val key = t.keys().filter(nId => nId.key_hash == b).fold({version: 0, key_hash: []}, (s, x) => x)
      t.get(key)


  /// If a node (different from the root) has one child, the child is an internal node
  pure def density(t: TreeMap): bool =
    t.keys().filter(nId => nId.key_hash != []).forall(nId => match t.get(nId) {
        | Internal(n) =>
          if (internalNodeHasChildren(n))
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

  /// Invariant: If a node (different from the root) has one child, the child is an internal node
  def densityInv =
    pruned_until.to(tree.treeVersion()).forall(v => density(treeAtVersion(tree,v)))


  /// returns a set of all prefixes of a list 
  pure def allPrefixes (l: List[a]): Set[List[a]] = 
    0.to(l.length()).map(i => l.slice(0,i))
  
  /// Invariant: For every node has predecessors with higer (or equal) version
  /// (This could be rewritten to talk about direct predecessors only)
  def versionInv = 
    // This invariant actually works on the whole tree rather than on a TreeMap
    tree.nodes.keys().forall(a =>
      allPrefixes(a.key_hash).forall(p =>
        tree.nodes.keys().exists(b =>
        p == b.key_hash and b.version >= a.version
      )))

  /// Every internal node must have at least one child with the same version
  def denseVersions(t: TreeMap): bool =
    t.keys().forall(nId => match t.get(nId) {
        | Internal(n) =>
          val leftOK = 
            match n.left_child {
              | Some(c) => t.keys().exists(a => a.key_hash == nId.key_hash.append(0) and a.version == nId.version)
              | None => false
            }  
          val rightOK = 
            match n.right_child {
              | Some(c) => t.keys().exists(a => a.key_hash == nId.key_hash.append(1) and a.version == nId.version)
              | None => false
            }  
          or(leftOK, rightOK)
        | Leaf(_) => true
    })

}


