module apply_state_machine {
  import basicSpells.* from "./spells/basicSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  import node.* from "./node"
  import apply_fancy.* from "./apply_fancy"

  pure val KEY_HASH_SIZE = 4

  var tree: Tree
  var version: int

  action init = all {
    // For now, we always start with an empty tree
    tree' = { nodes: Map(), orphans: Set() },
    version' = 1
  }

  action step = all {
    nondet key_hashes_as_maps = (0.to(KEY_HASH_SIZE).setOfMaps(Set(0, 1))).powerset().oneOf()
    pure val ops: Set[OperationOnKey] = key_hashes_as_maps.map(km => {
      pure val key_hash = range(0, KEY_HASH_SIZE).foldl([], (acc, i) => acc.append(km.get(i)))
      { key_hash: key_hash, op: Insert(key_hash) } // value_hash is equal to key_hash to make things easier
    })
    tree' = apply_fancy(tree, version - 1, version, ops),
    version' = version + 1
  }
}
