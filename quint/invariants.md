# Invariants

This part of the document describes invariants of Grug's JellyFish Merkle Tree manipulation. The snippets in here get tangled into a Quint file for verification.

<!--
```bluespec apply_state_machine.qnt +=
// -*- mode: Bluespec; -*-

module apply_state_machine {
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"

  import grug_ics23.* from "./grug_ics23"
  import proofs.* from "./proofs"

  import apply_simple as simple from "./apply_simple"
  import apply_fancy as fancy from "./apply_fancy"

  import completeness.* from "./completeness"
  import soundness.* from "./soundness"

  pure val VALUES = Set(Insert([1]), Insert([2]), Delete)

  var tree: Tree
  var version: int
  var smallest_unpruned_version: int
  var ops_history: List[Set[OperationOnKey]]

  action init = all {
    // For now, we always start with an empty tree
    tree' = { nodes: Map(), orphans: Set() },
    version' = 1,
    smallest_unpruned_version' = 1,
    ops_history' = [],
  }

  pure def to_operations(nondet_value: BitArray -> Operation): Set[OperationOnKey] = {
    nondet_value.mapToTuples().map(((key_hash, op)) => {
      { key_hash: key_hash, op: op }
    })
  }

  action step_parametrized(
    apply_op: (Tree, int, int, Set[OperationOnKey]) => Tree,
    assign_result: (Set[OperationOnKey], Tree) => bool
  ): bool = {
    nondet kms_with_value = all_key_hashes.setOfMaps(VALUES).oneOf()
    pure val all_ops = kms_with_value.to_operations().toList(fuzzy_compare)

    nondet threshold = 0.to(all_ops.length()).oneOf()
    pure val ops = all_ops.indices().filter(i => i < threshold).map(i => all_ops[i])
    pure val new_tree = apply_op(tree, version - 1, version, ops)

    assign_result(ops, new_tree)
  }

  action assign_result(ops: Set[OperationOnKey], new_tree: Tree): bool = all {
    tree' = new_tree,
    version' = version + 1,
    smallest_unpruned_version' = smallest_unpruned_version,
    ops_history' = ops_history.append(ops),
  }

  action step_fancy = step_parametrized(fancy::apply, assign_result)
  action step_simple = step_parametrized(simple::apply, assign_result)

  /********* INVARIANTS ***********/

```
-->

## Data structures

The main data structures are trees and nodes, defined below.

### Nodes

```bluespec
type Child = {
    version: int,
    hash: Term,
}

type InternalNode = {
    left_child: Option[Child],
    right_child: Option[Child],
}

type LeafNode = {
    // In the implementation it is a hash of a key but in the Radix tree it is
    // just used as a key, so we use a list of bits and we treat it here just as
    // bytes
    key_hash: Bytes,
    value_hash: Bytes,
}

type Node =
    | Internal(InternalNode)
    | Leaf(LeafNode)

```

### Trees

```bluespec
type BitArray = List[int]
type Version = int
type NodeId = { version: Version, key_hash: BitArray }

type OrphanId = {
  orphaned_since_version: Version,
  version: Version,
  key_hash: BitArray
}

type TreeMap = NodeId -> Node

type Tree = {
  nodes: TreeMap,
  orphans: Set[OrphanId]
}
```

## Projections of trees at versions

Some invariants can consider the entire tree with all the different versions living on it, but most invariants need to work with a single tree (as per the standard tree definition) at a time. Therefore, we need some auxiliary definitions to help getting a single tree's state at a given version, which we call a tree map.

```bluespec
  pure def treeAtVersion(t: Tree, version: int): TreeMap = {
    // take the nodes for the current version...
    val startingNodes = nodesAtVersion(t.nodes, version)
    // ... and grow the tree by adding direct children
    val nodePool = nodesUpToVersion(t.nodes, version)
    0.to(MAX_HASH_LENGTH).fold(startingNodes, (treeNodes, _) => {
      addDirectChildren(treeNodes, nodePool)
    })
  }
```

We also keep track of the smallest unpruned version, so we can check the invariant for the trees at all versions from that until the current one. We can switch between checking all of them or just the latest version at each state.

```bluespec apply_state_machine.qnt +=
  /// Which versions to check on the invariants
  /// This was checked both with Set(version) (better performance) and activeTreeVersions
  val versionsToCheck: Set[int] =
    Set(version)

  /// The set of unpruned tree versions that should be complete in the tree
  val activeTreeVersions: Set[int] =
    smallest_unpruned_version.to(tree.treeVersion())

  /// The set of tree maps to check
  val treesToCheck: Set[TreeMap] =
    versionsToCheck.map(v => treeAtVersion(tree, v))
```

<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

## Defined Invariants

### Parents of nodes exist in the tree

In a tree, all nodes except from the root should have a parent. There should be no dangling nodes. If we find a node with key 1001, there should be a node for 100. At some other iteration, we'll also check that 100 has a parent (that is, 10), and so on.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(everyNodesParentIsInTheTree)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Two leafs have a node with a common prefix

For any pair of leafs on the tree, there should be another node such that its key hash is the common prefix of the key hashes of both leafs.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  /// Invariant that checks that for any two leaf nodes, there is a nodeId in the
  /// tree that is the common prefix
  val nodeAtCommonPrefixInv: bool = {
    // Check whether there is a node with the given prefix
    pure def existsNode(t: TreeMap, b: Bytes): bool =
      t.keys().filter(nId => nId.key_hash == b).size() > 0

    // For any two leaf nodes, is there a nodeId in the tree that is the common prefix
    // of the nodes?
    pure def nodeAtCommonPrefix(t: TreeMap) : bool =
      t.allLeafs().forall(a =>
        t.allLeafs().forall(b =>
          (a.key_hash != b.key_hash) implies existsNode(t, commonPrefix(a, b))
      ))

    treesToCheck.forall(nodeAtCommonPrefix)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Leaf nodes should not be prefixes of any other node

Leafs should never be in the middle of a tree. If a there is a node with a key hash that is a prefix of the key hash of a leaf, then this is not a proper tree.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(noLeafInPrefixes)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Internal nodes have at least one child

Internal nodes can have 1 or 2 children, but never 0. Otherwise, they would be a leaf with no value.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(allInternalNodesHaveAChild)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Only children are internal nodes

This checks that collapsing works properly: there should never be an internal node with a single child where that child is a leaf. If it was a leaf, since there is no sibling, we should have collapsed it and make the internal node the leaf itself.

This also means that the three is dense, not sparse.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(isDense)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Versions of predecessors in path should be >= node version

When nodes are updated (inserted, deleted) they are given a new version, and so are all the predecessors in the tree. At the same time, subtrees that are not touched by an update maintain their version. As a result, if the tree is properly maintained, the parent of a node, should have a version that is greater than or equal to the version of the node.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Internal nodes share the version with at least one child

Given the explanation around `versionInv` from above, we had the (wrong) intuition, that since updates always push their version up the tree, every internal node should have the version of at least one of it children. However, the intuition is misleading:

- in the case of a delete, where a parent gets a new version, but there may not be a node at the spot where the deleted nodes had been.
- in the case of applying an empty batch, where we get a new root node at the new version, but the root node's subtrees are unchanged.
However, for reference, we keep the formula here as it might be useful for understanding in the future.

*Status:* FALSE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(denseVersions)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Orphans should not appear in trees after they are orphaned

Orphans are used for state pruning. The implicit intuition is that orphaned nodes can be pruned as they are not needed for any tree operation (including proof construction or verification) for versions after they became orphans. This invariant makes it explicit why it is safe to prune orphans: No orphan is part of a tree at a version after it became orphaned.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  /// Orphan should not be at one of the version trees after it gets orphaned
  val orphansInNoTreeInv: bool = {
    tree.orphans.forall(o =>
      val nodeId = { version: o.version, key_hash: o.key_hash }
      o.orphaned_since_version.to(tree.treeVersion()).forall(ver =>
        not(tree.treeAtVersion(ver).keys().contains(nodeId))))
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Stored hashes are actual hashes

For all internal nodes, for each existing child, the hash should match the result of hashing the subtree under that child's key hash.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(properlyHashed)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Stored hashes are unique

We use an implementation of hashes that ensure no collision, since no collision of hashes is an assumption we can make. This invariant is a sanity check that the hashes that get saved in the child nodes are unique.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
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

    treesToCheck.forall(uniqueHashes)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Node ids for tree maps have unique key hashes

While the overall tree receives an entry for the same `key_hash` whenever the corresponding value changes in a new version; in the versioned tree, each node should contain at most one entry for each `key_hash`.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  /// The treemaps we use in the invariants should have unique key_hashes
  /// This is a sanity check for the treeAtVersion computation
  val goodTreeMapInv: bool = {
    pure def goodTreeMap(t: TreeMap): bool =
      t.keys().forall(a =>
        t.keys().forall(b => a.key_hash == b.key_hash implies a.version == b.version))

    treesToCheck.forall(goodTreeMap)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Tree maps are bijective

Just as `key_hashes` in the invariant above, all nodes saved in the values of a tree map should be unique. This also mean that the mapping between node ids and nodes is bijective.
We can simply check that the sizes of keys and values is the same, since keys are already guaranteed to be unique by Quint's `Map` data structure.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  /// TreeMap is a bijection: we can map keys to values but also values to keys
  /// In other words, values are also unique
  /// Only works on tree map, as trees can have same nodes in different keys/versions
  val bijectiveTreeMapInv: bool = {
    pure def bijectiveTreeMap(t: TreeMap): bool =
      t.keys().size() == t.values().size()

    treesToCheck.forall(bijectiveTreeMap)
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Operations are properly applied

TODO: describe

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  val operationSuccessInv: bool =
    pure def treeContainsKV(t: TreeMap, n: LeafNode): bool =
      t.values().contains(Leaf(n))

    pure def treeNotContainsKey(t: TreeMap, key: BitArray): bool =
      t.values()
        .filter(node =>
          match node {
            | Leaf(n) => n.key_hash == key
            | Internal(_) => false
        })
        .size() == 0

    val tm = tree.treeAtVersion(version - 1)
    ops_history.length() > 0 implies
      ops_history.last().forall(op => {
        match op.op {
          | Insert(value) => treeContainsKV(tm, { key_hash: op.key_hash, value_hash: value })
          | Delete => treeNotContainsKey(tm, op.key_hash)
        }
      })
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

#### Completeness and Soundness

These are invariants for completeness and soundness of both membership and non-membership proofs.

Completeness:
    - Membership: If a node exists, we should be able to get an existence proof for it and verify that proof against the tree root
    - Non-Membership: If a node does not exist, we should be able to get a non-existence proof for it and verify that proof against the tree root

Soundness:

  - Membership: If we can get an existence proof for a key_hash, there should be a leaf in the tree with that key hash.
  - Non-Membership: If we can get a non-existence proof for a key_hash, there should not be a leaf in the tree with that key hash.

See [completeness.qnt](completeness.qnt) and [soundness.qnt](soundness.qnt) for the formal definitions.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  val membershipCompletenessInv = versionsToCheck.forall(v => membershipCompleteness(tree, v))
  val nonMembershipCompletenessInv = versionsToCheck.forall(v => nonMembershipCompleteness(tree, v))
  val membershipSoundnessInv = versionsToCheck.forall(v => membershipSoundness(tree, v))
  val nonMembershipSoundnessInv = versionsToCheck.forall(v => nonMembershipSoundness(tree, v))
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

### Proofs are only verified for their `key_hash`

This invariant creates proofs for all possible combinations of `key_hashes`. Some of them will be existence proofs and some will be non-existence.

- For existence proofs:
  - There should be a leaf in the tree with that `key_hash` and it the proof should be verified with this leaf's `key_hash` and `value_hash`
    - It should not be verified with any other `value_hash`
  - It should not be verified for all other `key_hashe`es (with any `value_hash`)
we consider all possible values for `value_hash`. If there is a leaf in the tree with that `key_hash` + `value_hash` combination, the proof should be verified against the tree, but not otherwise.
- For non-existence proofs:
  - It should be verified for the `key_hash` it was proved with
  - It should not be verified with any of the `key_hash`es in the tree.

*Status:* TRUE

```bluespec apply_state_machine.qnt +=
  val verifyMembershipInv = {
    versionsToCheck.forall(version => {
      val leafs = tree.treeAtVersion(version).allLeafs()
      val root = hash(tree.nodes.get({ key_hash: ROOT_BITS, version: version }))

      tree.nodes.has({ key_hash: ROOT_BITS, version: version }) implies
        all_key_hashes.forall(key_hash => {
          val proof = ics23_prove(tree, key_hash, version)
          match proof {
            | Some(p) =>
              match p {
                | Exist(ep) => and {
                  leafs.exists(l => and {
                    // There should be a leaf with this key_hash
                    l.key_hash == key_hash,

                    // and verifying the proof with its value_hash should work,
                    verifyMembership(root, ep, l.key_hash, l.value_hash),

                    // while verifying with any other value_hash should fail
                    all_value_hashes.exclude(Set(l.value_hash)).forall(value_hash => {
                      not(verifyMembership(root, ep, key_hash, value_hash))
                    })
                  }),

                  // Verifying the proof against all other key_hashes and value_hashes should fail
                  all_key_hashes.exclude(Set(key_hash)).forall(key_hash => {
                    all_value_hashes.forall(value_hash => {
                      not(verifyMembership(root, ep, key_hash, value_hash))
                    })
                  }),
                }
                | NonExist(nep) => and {
                  // Verifying the proof against this key_hash should work
                  verifyNonMembership(root, nep, key_hash),
                  // Verifying the proof against all other key_hashes should fail
                  leafs.forall(l => {
                    not(verifyNonMembership(root, nep, l.key_hash))
                  }),
                }
              }
            | None => true  // corresponds to the panic in the rust code comment on the line 44 ics23.rs
          }
      })
    })
  }
```
<!--
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

<!--
```bluespec apply_state_machine.qnt +=
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
    if (operationSuccessInv) true else q::debug("operationSuccessInv", false),
    if (membershipCompletenessInv) true else q::debug("membershipCompletenessInv", false),
    if (nonMembershipCompletenessInv) true else q::debug("nonMembershipCompletenessInv", false),
    if (membershipSoundnessInv) true else q::debug("membershipSoundnessInv", false),
    if (nonMembershipSoundnessInv) true else q::debug("nonMembershipSoundnessInv", false),
    if (verifyMembershipInv) true else q::debug("verifyMembershipInv", false),
  }
}
```
-->

# Tests

This part of the document describes relevant tests for tree manipulation and ICS23 proofs of Grug's JellyFish Merkle Tree manipulation. The snippets in here get tangled into a Quint files for verification.

## Testing functional equivalence

We defined two runs that are verifying the functional equivalence of [`apply_fancy`](./apply_fancy.qnt) and [`apply_simple`](./apply_simple.qnt) algorithms:

- [`simpleVsFancyTest`](#simple-vs-fancy-test)
- [`simpleVsFancyMultipleRepsTest`](#simple-vs-fancy-multiple-reps-test)

<!--- 
```bluespec test/tree_test.qnt +=
// -*- mode: Bluespec; -*-

module tree_test {
  import tree.* from "../tree"
  import node.* from "../node"
  import utils.* from "../utils"
  import basicSpells.* from "../spells/basicSpells"
  import apply_state_machine.* from "../apply_state_machine"
  import apply_fancy as fancy from "../apply_fancy"
  import apply_simple as simple from "../apply_simple"

  <<<tests>>>
}
```
--->

### Simple Vs Fancy Test

Goal of this test is to compare the outcome of `apply_simple` and `apply_fancy` algorithms on an empty tree after one batch.

First, we create an empty tree.

```bluespec "tests" +=
run simpleVsFancyTest =
  pure val empty_tree = { nodes: Map(), orphans: Set() }
```

Then we create a batch of randomly picked operations to apply on both trees.

```bluespec "tests" +=
  nondet kms_with_value = all_key_hashes.setOfMaps(VALUES).oneOf()
  pure val ops = kms_with_value.to_operations()
```
<!--- 
```bluespec "tests" +=

```
--->
We create `reference` tree using `apply_simple` algorithm, and `result` tree using `apply_fancy`.

```bluespec "tests" +=
  pure val reference = simple::apply(empty_tree, 0, 1, ops)
  pure val result = fancy::apply(empty_tree, 0, 1, ops)
```

Then, we assert their equivalence.

```bluespec "tests" +=
  assert(reference == result)
```
<!--- 
```bluespec "tests" +=

```
--->

### Simple Vs Fancy Multiple Reps Test

Goal of this test is to compare the outcome of `apply_simple` and `apply_fancy` algorithms on an empty tree after 3 batches. This test utilizes already existing state machine.
First we call `action init`.

```bluespec "tests" +=
run simpleVsFancyMultipleRepsTest = init.then(3.reps(_ => {
```

After that, we repeat the following set of operations three times. First we pick randomly a set of operations that will be in a batch.

```bluespec "tests" +=
  nondet kms_with_value = all_key_hashes.setOfMaps(VALUES).oneOf()
  pure val ops = kms_with_value.to_operations()
```
<!--- 
```bluespec "tests" +=

```
--->
We create `reference` tree using `apply_simple` algorithm, and `result` tree using `apply_fancy`. We are using state variable `version` to indicate which `version` are we on currently. We created `failure = reference != result` variable to know if `reference` and `result` are not the same.

```bluespec "tests" +=
  val reference = simple::apply(tree, version - 1, version, ops)
  val result = fancy::apply(tree, version - 1, version, ops)
  val failure = reference != result
```

Then we update the state using operator `all`, which means that everyhing wrapped by it has to be `true` in order to execute it. If there has been a failure, `assert(q::debug("simple", reference) == q::debug("fancy", result))` will return false, and the whole run would fail. `q::debug` statements are added so we can see relevant information if the test fails.

```bluespec "tests" +=
  all {
    tree' = result,
    version' = version + 1,
    ops_history' = if (failure) q::debug("ops_history", ops_history.append(ops)) else ops_history.append(ops),
    smallest_unpruned_version' = smallest_unpruned_version,
    if (failure) assert(q::debug("simple", reference) == q::debug("fancy", result)) else true,
  }
}))
```

## Testing proofs

We defined several interesting scenarios in order to test proofs.
<!--- 
```bluespec test/proofs_test.qnt +=
// -*- mode: Bluespec; -*-

module proofs_test {
  import proofs.* from "../proofs"
  import basicSpells.* from "../spells/basicSpells"
  import apply_simple.* from "../apply_simple"
  import rareSpells.* from "../spells/rareSpells"
  import node.* from "../node"
  import tree.* from "../tree"
  import proof_types.* from "../proof_types"
  import grug_ics23.* from "../grug_ics23"
  import utils.* from "../utils"

  import instantiatable_tree(fancy=false) as T1 from "instantiatable_tree"
  import instantiatable_tree(fancy=false) as T2 from "instantiatable_tree"

  val empty_tree: Tree = { nodes: Map(), orphans: Set() }

  <<<proofs_helpers>>>
  <<<proofs>>>
}
```
--->
- Let `t1` and `t2` be different trees, and let `p` be a (non)membership proof from `t1`. Proof `p` should not be verifiable against the root of tree `t2`.
<!---
```bluespec "proofs" +=
/// If t1 and t2 are different, verifying proofs from t1 against t2 root should fail.
```
--->
```bluespec "proofs" +=
run twoDifferentTreesTest = generate_two_trees.expect({
  nondet version = 1.to(max_version).oneOf()

  val leafs1 = tree1.treeAtVersion(version).allLeafs()
  val leafs2 = tree2.treeAtVersion(version).allLeafs()

  and {
    not(leafs1.empty()),
    not(leafs2.empty()),
    leafs1 != leafs2
  } implies
    // TODO: This should hold for non-existence proofs as well, but we only check existence
    nondet leaf = leafs1.oneOf()

    assert_proof_on_different_trees(tree1, tree2, leaf, version)
})
```

- Let `t1` and `t2` be trees that have the same keys, but different values, and let `p` be a (non)membership proof from `t1`. Proof `p` should not be verifiable against the root of tree `t2`.
<!---
```bluespec "proofs" +=

/// If t1 and t2 have the same keys but different values, verifying proofs
/// from t1 against t2 root should fail.
```
--->
```bluespec "proofs" +=
run twoDifferentTreesByOnlyValuesTest = generate_one_tree.expect({
  nondet version = 1.to(max_version).oneOf()
  val leafs1 = tree1.treeAtVersion(version).allLeafs()

  not(leafs1.empty()) implies
    val ops_with_different_values = leafs1.map(kv => { key_hash: kv.key_hash, op: Insert([kv.value_hash[0] + 1]) })
    val tree_with_different_values = empty_tree.apply(0, version, ops_with_different_values)

    nondet leaf = leafs1.oneOf()
    assert_proof_on_different_trees(tree1, tree_with_different_values, leaf, version)
})
```

- Let `t1` and `t2` be different trees with only one key-value pair being the same, and let `p` be a (non)membership proof from `t1`. Proof `p` should not be verifiable against the root of tree `t2`.
<!---
```bluespec "proofs" +=

/// If t1 and t2 are different but have one leaf in common, verifying a proof
/// from t1 for that leaf against t2 root should fail.
```
--->
```bluespec "proofs" +=
run twoDifferentTreesSameByOnlyOneKVTest = generate_two_trees.expect({
  val leafs1 = tree1.treeAtVersion(max_version).allLeafs()
  val leafs2 = tree2.treeAtVersion(max_version).allLeafs()

  and {
    not(leafs1.empty()),
    not(leafs2.empty()),
    leafs1 != leafs2
  } implies
    nondet leaf = leafs1.oneOf()
    val op = { key_hash: leaf.key_hash, op: Insert(leaf.value_hash) }

    val t1 = tree1.apply(max_version, max_version + 1, Set())
    val t2 = tree2.apply(max_version, max_version + 1, Set(op))

    assert_proof_on_different_trees(t1, t2, leaf, max_version + 1)
})
```

- Let `t1` and `t2` be different trees by only one, and let `p` be a (non)membership proof from `t1`. Proof `p` should not be verifiable against the root of tree `t2`.
<!---
```bluespec "proofs" +=

/// If t1 and t2 have the same keys but a single leaf with a different value,
/// verifying proofs from t1 against t2 root should fail.
```
--->
```bluespec "proofs" +=
run twoDifferentTreesByOnlyOneValueTest = generate_one_tree.expect({
  nondet version = 1.to(max_version).oneOf()
  val leafs1 = tree1.treeAtVersion(version).allLeafs()

  not(leafs1.empty()) implies
    nondet leaf_to_change = leafs1.oneOf()
    val ops_with_different_values = leafs1.map(kv => {
      key_hash: kv.key_hash,
      op: if (kv == leaf_to_change) Insert([kv.value_hash[0] + 1]) else Insert(kv.value_hash)
    })
    val tree_with_different_values = empty_tree.apply(0, version, ops_with_different_values)

    nondet leaf = leafs1.oneOf()
    assert_proof_on_different_trees(tree1, tree_with_different_values, leaf, version)
})
```

- Let `t2` be a prunned version of `t1`, and let `p` be a membership proof from `t1` for an arbitrary leaf `l`. Proof `p` should be verifiable against the root of tree `t2` as long as leaf `l` is not prunned in `t2`.
<!---
```bluespec "proofs" +=

/// An existence proof for a tree should work on that tree after prunning, as
/// long as the leaf is not prunned.
```
--->
```bluespec "proofs" +=
run verificationOnPrunnedTreeTest = generate_one_tree.expect({
  nondet version = 1.to(max_version - 1).oneOf()
  val prunned_tree = tree1.prune(version)
  val active_leafs = prunned_tree.treeAtVersion(version).allLeafs()

  not(active_leafs.empty()) implies
    nondet leaf = active_leafs.oneOf()
    assert_proof_on_equivalent_trees(tree1, prunned_tree, leaf, version)
})
```

- Let `t` be an arbitrary tree with version `v`. Let `np` be a NonExistence proof for an arbitrary non-existant leaf `l`. Let `t'` be a tree `t` after inserting leaf `l`. Proof `np` should no be verifiable on tree `t'` and version `v+1`.
<!---
```bluespec "proofs" +=

/// If we have a non-existence proof for a leaf and then we add it to the
/// tree, the proof should not be verified anymore.
```
--->
```bluespec "proofs" +=
run leafNotExistsThenExistsTest = generate_one_tree.expect({
  val leafs1 = tree1.treeAtVersion(max_version).allLeafs()
  leafs1.size() < all_key_hashes.size() implies
    nondet non_existent_key_hash = all_key_hashes.exclude(leafs1.map(leaf => leaf.key_hash)).oneOf()

    val proof = ics23_prove(tree1, non_existent_key_hash, max_version).unwrap()

    match proof {
      | NonExist(nep) => {
        val updated_tree = tree1.apply(max_version, max_version + 1, Set({ key_hash: non_existent_key_hash, op: Insert([1]) }))
        val updated_tree_hash = hash(updated_tree.nodes.get({ key_hash: ROOT_BITS, version: max_version + 1 }))
        // We added the leaf, so the non-existence proof should not be verified anymore
        assert(not(verifyNonMembership(updated_tree_hash, nep, non_existent_key_hash)))
      }
      | _ => assert(false)
    }
})
```

- Let `t` be an arbitrary tree with version `v`. Let `p` be an Existence proof for an arbitrary existant leaf `l`. Let `t'` be a tree `t` after deleting leaf `l`. Proof `p` should no be verifiable on tree `t'` and version `v+1`.
<!---
```bluespec "proofs" +=

/// If we have an existence proof for a leaf and then we delete it from the
/// tree, the proof should not be verified anymore.
```
--->
```bluespec "proofs" +=
run leafExistsThenNotExistsTest = generate_one_tree.expect({
  val leafs1 = tree1.treeAtVersion(max_version).allLeafs()
  nondet leaf = leafs1.oneOf()

  val proof = ics23_prove(tree1, leaf.key_hash, max_version).unwrap()
  match proof {
    | Exist(ep) => {
      val updated_tree = tree1.apply(max_version, max_version + 1, Set({ key_hash: leaf.key_hash, op: Delete }))
      updated_tree.nodes.has({ key_hash: ROOT_BITS, version: max_version + 1 }) implies {
        val updated_tree_hash = hash(updated_tree.nodes.get({ key_hash: ROOT_BITS, version: max_version + 1 }))
        // We deleted the leaf, so the existence proof should not be verified anymore
        assert(not(verifyMembership(updated_tree_hash, ep, leaf.key_hash, leaf.value_hash)))
      }
    }
    | _ => assert(false)
  }
})
```

In order to do so, we have implemented two helper assert functions:

- `assert_proof_on_different_trees`

```bluespec "proofs_helpers" +=
pure def assert_proof_on_different_trees(t1: Tree, t2: Tree, leaf: LeafNode, version: Version): bool = {
  val proof = ics23_prove(t1, leaf.key_hash, version).unwrap()

  val root1_hash = hash(t1.nodes.get({ key_hash: ROOT_BITS, version: version }))
  val root2_hash = hash(t2.nodes.get({ key_hash: ROOT_BITS, version: version }))

  match proof {
    | Exist(ep) => all {
      // It should be verified on tree1 but not on tree2
      assert(verifyMembership(root1_hash, ep, leaf.key_hash, leaf.value_hash)),
      assert(not(verifyMembership(root2_hash, ep, leaf.key_hash, leaf.value_hash))),
    }
    | NonExist(nep) => true
  }
}
```
<!--- 
```bluespec "proofs_helpers" +=

```
--->
- `assert_proof_on_equivalent_trees`

```bluespec "proofs_helpers" +=
pure def assert_proof_on_equivalent_trees(t1: Tree, t2: Tree, leaf: LeafNode, version: Version): bool = {
  val proof1 = ics23_prove(t1, leaf.key_hash, version).unwrap()
  val proof2 = ics23_prove(t2, leaf.key_hash, version).unwrap()

  val root1_hash = hash(t1.nodes.get({ key_hash: ROOT_BITS, version: version }))
  val root2_hash = hash(t2.nodes.get({ key_hash: ROOT_BITS, version: version }))

  and {
    match proof1 {
      | Exist(ep) => all {
        // It should be verified on both trees
        assert(verifyMembership(root1_hash, ep, leaf.key_hash, leaf.value_hash)),
        assert(verifyMembership(root2_hash, ep, leaf.key_hash, leaf.value_hash)),
      }
      | NonExist(nep) => assert(false)
    },
    match proof2 {
      | Exist(ep) => all {
        // It should be verified on both trees
        assert(verifyMembership(root1_hash, ep, leaf.key_hash, leaf.value_hash)),
        assert(verifyMembership(root2_hash, ep, leaf.key_hash, leaf.value_hash)),
      }
      | NonExist(nep) => assert(false)
    }
  }
}
```
<!--- 
```bluespec "proofs_helpers" +=

```
--->
We also implemented two runs that utilize already existing state machine:

- `generate_one_tree`

```bluespec "proofs_helpers" +=
run generate_one_tree = ({
  T1::init
}).then(3.reps(_ => {
  T1::step
}))
```
<!--- 
```bluespec "proofs_helpers" +=

```
--->
- `generate_two_trees`

```bluespec "proofs_helpers" +=
run generate_two_trees = (all {
  T1::init,
  T2::init,
}).then(3.reps(_ => all {
  T1::step,
  T2::step,
}))
```
<!--- 
```bluespec "proofs_helpers" +=

// Some values to make tests easier to read
```
--->
We defined some values to make tests easier to read:

```bluespec "proofs_helpers" +=
val tree1 = T1::tree
val tree2 = T2::tree
val max_version = T1::version - 1
```
<!--- 
```bluespec "proofs_helpers" +=

```
--->
