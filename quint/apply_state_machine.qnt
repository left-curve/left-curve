// -*- mode: Bluespec; -*-

module apply_state_machine {
  import basicSpells.* from "./spells/basicSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"

  import apply_simple as simple from "./apply_simple"
  import apply_fancy as fancy from "./apply_fancy"
  import apply_super_simple as super_simple from "./apply_super_simple"

  pure val MAX_OPS = 5
  // Try to pick values with a balanced chance of collision
  // Operation will be Delete if value between 16 and 20
  // Each value has an id so Deletes don't become a single Delete when we construct the set.
  // We want multiple deletes to preserve probability (for the simulator)
  pure val VALUES = 2.to(20).map(v => { id: v, op: if (v > 15) Delete else Insert([v]) })

  var tree: Tree
  var version: int
  var smallest_unpruned_version: int
  var ops_history: List[Set[OperationOnKey]]

  action init = all {
    // For now, we always start with an empty tree
    tree' = { nodes: Map(), orphans: Set() },
    version' = 1,
    smallest_unpruned_version' = 0,
    ops_history' = [],
  }

  pure val all_key_hashes_as_maps = (0.to(MAX_HASH_LENGTH - 1).setOfMaps(Set(0, 1))).powerset()
  pure def key_hash_map_to_op(km: (int -> int, { id: int, op: Operation })): OperationOnKey = {
    pure val key_hash: Bytes_t = range(0, MAX_HASH_LENGTH).foldl([], (acc, i) => acc.append(km._1.get(i)))
    { key_hash: key_hash, op: km._2.op }
  }

  pure def to_operations(nondet_value: (int -> int) -> { id: int, op: Operation }): Set[OperationOnKey] = {
    nondet_value.mapToTuples().take(MAX_OPS).map(key_hash_map_to_op)
  }

  action step_parametrized(
    apply_op: (Tree, int, int, Set[OperationOnKey]) => Tree,
    assign_result: (Set[OperationOnKey], Tree) => bool
  ): bool = {
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()
    pure val ops = kms_with_value.to_operations()
    pure val new_tree = apply_op(tree, version - 1, version, ops)
    all {
      key_hashes_as_maps != Set(),
      assign_result(ops, new_tree)
    }
  }

  action assign_result(ops: Set[OperationOnKey], new_tree: Tree): bool = all {
    tree' = new_tree,
    version' = version + 1,
    smallest_unpruned_version' = smallest_unpruned_version,
    ops_history' = ops_history.append(ops),
  }

  action step_fancy = step_parametrized(fancy::apply, assign_result)
  action step_simple = step_parametrized(simple::apply, assign_result)
  action step_super_simple = step_parametrized(super_simple::apply, assign_result)

  /********* INVARIANTS ***********/

  /// The set of unpruned tree versions that should be complete in the tree
  val activeTreeVersions: Set[int] =
    smallest_unpruned_version.to(tree.treeVersion())

  /// The set of active tree maps
  val activeVersionedTrees: Set[TreeMap] =
    activeTreeVersions.map(v => treeAtVersion(tree, v))

  /// Make sure the tree encoded in the map forms a tree (everyone has a parent, except for the root)
  /// E.g., if a node has keyhash_prefix [1,0,0,1] then there must be a node with keyhash_prefix [1,0,0]
  val everyNodesParentIsInTheTreeInv: bool = {
    pure def everyNodesParentIsInTheTree(t: TreeMap): bool = {
      val prefixes = t.keys().map(p => p.key_hash)
      prefixes.filter(p => p != []).forall(p => {
        val parent = p.slice(0, p.length() - 1)
        prefixes.contains(parent)
      })
    }

    activeVersionedTrees.forall(everyNodesParentIsInTheTree)
  }

  /// Invariant that checks that for any two leaf nodes, there is a nodeId in the
  /// tree that is the common prefix
  val nodeAtCommonPrefixInv: bool = {
    // Check whether there is a node with the given prefix
    pure def existsNode(t: TreeMap, b: Bytes_t): bool =
      t.keys().filter(nId => nId.key_hash == b).size() > 0

    // For any two leaf nodes, is there a nodeId in the tree that is the common prefix
    // of the nodes?
    pure def nodeAtCommonPrefix(t: TreeMap) : bool =
      t.allLeafs().forall(a =>
        t.allLeafs().forall(b =>
          (a.key_hash != b.key_hash) implies existsNode(t, commonPrefix(a, b))
      ))

    activeVersionedTrees.forall(nodeAtCommonPrefix)
  }

  /// Make sure that the map encodes a tree. In particular, there is no internal node
  /// that has a leaf node as its prefix
  /// E.g., the map is not allowed to have: [0,0,1] -> internal node; [0,0] -> leaf node
  val noLeafInPrefixesInv: bool = {
    pure def noLeafInPrefixes(t: TreeMap): bool = {
      val nodes = t.keys().map(nId => nId.key_hash)
      val leafs = t.keys().filter(nId => t.get(nId).isLeaf()).map(nId => nId.key_hash)

      nodes.forall(node => {
        not(leafs.exists(leaf => node != leaf and leaf.prefix_of(node)))
      })
    }

    activeVersionedTrees.forall(noLeafInPrefixes)
  }

  /// All nodes of type Internal have a child which is not None
  val allInternalNodesHaveAChildInv: bool = {
    pure def internalNodeHasAChild(n: InternalNode): bool = {
      n.left_child != None or n.right_child != None
    }

    pure def allInternalNodesHaveAChild(t: TreeMap): bool = {
      t.keys().forall(nId => {
        match t.get(nId) {
          | Internal(n) => internalNodeHasAChild(n)
          | Leaf(_) => true
        }
      })
    }

    activeVersionedTrees.forall(allInternalNodesHaveAChild)
  }

  /// If a node has exactly one child, the child is an internal node
  /// (if it was a leaf, then the node itself would be the leaf)
  val densityInv: bool = {
    pure def isDense(t: TreeMap): bool =
      t.keys().forall(nId => {
        match t.get(nId) {
          | Internal(n) =>
            if (n.left_child == None and n.right_child != None) {
              // Only has right child, right child should be internal
              findNode(t, nId.key_hash.append(1)).isInternal()
            } else if (n.right_child == None and n.left_child != None) {
              // Only has left child, left child should be internal
              findNode(t, nId.key_hash.append(0)).isInternal()
            } else {
              // Has two children or none
              true
            }
          | Leaf(_) => true
        }
      })

    activeVersionedTrees.forall(isDense)
  }

  /// Invariant: For every node has predecessors with higer (or equal) version
  /// (This could be rewritten to talk about direct predecessors only)
  val versionInv: bool = {
    pure def allPrefixes (l: List[a]): Set[List[a]] =
      0.to(l.length()).map(i => l.slice(0, i))

    // This invariant actually works on the whole tree rather than on a TreeMap
    tree.nodes.keys().forall(a =>
      allPrefixes(a.key_hash).forall(p =>
        tree.nodes.keys().exists(b =>
          p == b.key_hash and b.version >= a.version)))
  }

  /// Every internal node must have at least one child with the same version
  /// This doesn't hold - should it?
  /// TODO: check why this doesn't hold even when there are no deletions (seed 0x61ec6acbe4eda)
  val denseVersionsInv: bool = {
    def denseVersions(t: TreeMap): bool = {
      t.keys().forall(nId => {
        match t.get(nId) {
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
        }
      })
    }

    activeVersionedTrees.forall(denseVersions)
  }

  /// Orphan should not be at one of the version trees after it gets orphaned
  val orphansInNoTreeInv: bool = {
    tree.orphans.forall(o =>
      val nodeId = { version: o.version, key_hash: o.key_hash }
      o.orphaned_since_version.to(tree.treeVersion()).forall(ver =>
        not(tree.treeAtVersion(ver).keys().contains(nodeId))))
  }

  /// Check that for all internal nodes, if they have a hash stored for a child,
  /// then the hash is the hash of the actualy the subtree for the child
  val hashInv: bool = {
    pure def properlyHashed(t: TreeMap): bool = {
      t.keys().forall(nID => {
        match t.get(nID) {
          | Leaf(_) => true
          | Internal(n) => {
            match n.left_child {
              | None => true
              | Some(c) => c.hash == hash(t.findNode(nID.key_hash.append(0)))
            }
            and
            match n.right_child {
              | None => true
              | Some(c) => c.hash == hash(t.findNode(nID.key_hash.append(1)))
            }
          }
        }
      })
    }

    activeVersionedTrees.forall(properlyHashed)
  }

  /// All children in the tree should have different hashes
  val uniqueHashesInv: bool = {
    pure def uniqueHashes(t: TreeMap): bool = {
      pure val hashes = t.values().fold([], (acc, node) => {
        match node {
          | Internal(n) => {
            pure val acc_1 = match n.left_child {
              | None => acc
              | Some(c) => acc.append(c.hash)
            }
            match n.right_child {
              | None => acc_1
              | Some(c) => acc_1.append(c.hash)
            }
          }
         | _ => acc
        }
      })
      pure val uniqueHashes = hashes.foldl(Set(), (acc, hash) => {
        acc.union(Set(hash))
      })

      hashes.length() == uniqueHashes.size()
    }

    activeVersionedTrees.forall(uniqueHashes)
  }

  /// The treemaps we use in the invariants should have unique key_hashes
  /// This is a sanity check for the treeAtVersion computation
  val goodTreeMapInv: bool = {
    pure def goodTreeMap(t: TreeMap): bool =
      t.keys().forall(a =>
        t.keys().forall(b => a.key_hash == b.key_hash implies a.version == b.version))

    activeVersionedTrees.forall(goodTreeMap)
  }

  /// TreeMap is a bijection: we can map keys to values but also values to keys
  /// In other words, values are also unique
  /// Only works on tree map, as trees can have same nodes in different keys/versions
  val bijectiveTreeMapInv: bool = {
    pure def bijectiveTreeMap(t: TreeMap): bool =
      t.keys().size() == t.values().size()

    activeVersionedTrees.forall(bijectiveTreeMap)
  }

  val allInvariants = all {
    if (everyNodesParentIsInTheTreeInv) true else q::debug("everyNodesParentIsInTheTreeInv", false),
    if (nodeAtCommonPrefixInv) true else q::debug("nodeAtCommonPrefixInv", false),
    if (noLeafInPrefixesInv) true else q::debug("noLeafInPrefixesInv", false),
    if (allInternalNodesHaveAChildInv) true else q::debug("allInternalNodesHaveAChild", false),
    if (densityInv) true else q::debug("densityInv", false),
    if (versionInv) true else q::debug("versionInv", false),
    if (orphansInNoTreeInv) true else q::debug("orphansInNoTreeInv", false),
    if (hashInv) true else q::debug("hashInv", false),
    if (uniqueHashesInv) true else q::debug("uniqueHashesInv", false),
    if (goodTreeMapInv) true else q::debug("goodTreeMapInv", false),
    if (bijectiveTreeMapInv) true else q::debug("bijectiveTreeMapInv", false),
  }
}

module pruning_state_machine {
  import tree.* from "./tree"
  import apply_fancy as fancy from "./apply_fancy"

  import apply_state_machine as base
  import apply_simple as simple from "./apply_simple"
  import apply_fancy as fancy from "./apply_fancy"
  import apply_super_simple as super_simple from "./apply_super_simple"

  var unpruned_tree: Tree

  action init = all {
    base::init,
    unpruned_tree' = { nodes: Map(), orphans: Set() },
  }

  action assign_result(ops, new_tree) = all {
    base::assign_result(ops, new_tree),
    unpruned_tree' = new_tree,
  }

  action prune_action = all {
    base::version > base::smallest_unpruned_version,
    nondet version_to_prune = base::smallest_unpruned_version.to(base::version - 1).oneOf()
    all {
      base::tree' = base::tree.prune(version_to_prune),
      base::version' = base::version,
      base::smallest_unpruned_version' = version_to_prune + 1,
      base::ops_history' = base::ops_history,
      unpruned_tree' = unpruned_tree,
    }
  }

  action step_fancy = any {
    prune_action,
    base::step_parametrized(fancy::apply, assign_result)
  }

  action step_simple = any {
    prune_action,
    base::step_parametrized(simple::apply, assign_result)
  }

  action step_super_simple = any {
    prune_action,
    base::step_parametrized(super_simple::apply, assign_result)
  }

  def pruningDoesNotAffectActiveTreesInv =
    base::activeTreeVersions.forall(v =>
      base::tree.treeAtVersion(v) == unpruned_tree.treeAtVersion(v)
    )

  val allInvariants = all {
    base::allInvariants,
    if(pruningDoesNotAffectActiveTreesInv) true else q::debug("pruningDoesNotAffectActiveTreesInv", false),
  }
}

