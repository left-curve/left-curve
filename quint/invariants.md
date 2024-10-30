# Invariants

This document describes invariants of grug's jellyfish merkle tree manipulation.

<!-- 
```bluespec apply_state_machine.qnt +=
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

  action step_fancy = {
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()
    pure val ops = kms_with_value.to_operations()
    all {
      tree' = fancy::apply(tree, version - 1, version, ops),
      version' = version + 1,
      smallest_unpruned_version' = smallest_unpruned_version,
      ops_history' = ops_history.append(ops),
    }
  }

  action step_simple = {
    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()
    pure val ops = kms_with_value.to_operations()
    pure val ops = kms_with_value.keys().map(k => (k, kms_with_value.get(k))).take(MAX_OPS).map(key_hash_map_to_op)
    all {
      tree' = simple::apply(tree, version - 1, version, ops),
      version' = version + 1,
      smallest_unpruned_version' = smallest_unpruned_version,
      ops_history' = ops_history.append(ops),
    }
  }

  /********* INVARIANTS ***********/

```
-->

## Data structures
- TODO

## Projections of trees at versions

We use some auxiliary definitions to obtain a tree at a given version
- TODO: explain this better


```bluespec apply_state_machine.qnt +=
  /// The set of unpruned tree versions that should be complete in the tree 
  def activeTreeVersions: Set[int] =
    smallest_unpruned_version.to(tree.treeVersion())
```

<!-- 
This inserts a line break that is not rendered in the markdown
```bluespec apply_state_machine.qnt +=

```
-->

## Invariants

### Parents of nodes exist in the tree

- TODO: describe

```bluespec apply_state_machine.qnt +=
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
    activeTreeVersions.forall(v =>
      val vTree = treeAtVersion(tree,v)
      everyNodesParentIsInTheTree(vTree)
    )
```

<!-- 
```bluespec apply_state_machine.qnt +=
}
```
-->
