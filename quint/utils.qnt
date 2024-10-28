// -*- mode: Bluespec; -*-

module utils {
  import basicSpells.* from "./spells/basicSpells"
  import node.* from "./node"
  import hashes.* from "./hashes"
  val keylength = 4

  pure val DEBUG_ENABLED = false

  pure def debug(desc, expr) = if (DEBUG_ENABLED) q::debug(desc, expr) else expr

  pure def unwrap(value: Option[a]): a = {
    match value {
      | None => Map().get(value)
      | Some(x) => x
    }
  }

  pure def commonPrefixBetweenKeys(a: List[int], b: List[int]) : List[int] =
    val indList = range(1, keylength + 1)
    indList.foldl([], (s, x) => if (a.slice(0, x) == b.slice(0, x)) 
                                    b.slice(0, x) 
                                else s )

  pure def commonPrefix(a: LeafNode, b: LeafNode) : Bytes_t =
    val indList = range(1, keylength + 1)
    indList.foldl([], (s, x) => if (a.key_hash.slice(0, x) == b.key_hash.slice(0, x)) 
                                    b.key_hash.slice(0, x) 
                                else s )
  
  pure def prefix_of(l1: List[a], l2: List[b]): bool = and {
    l1.length() <= l2.length(),
    l1.indices().forall(i => l1[i] == l2[i])
  }

  // Warning: don't assume this is either deterministic or non-deterministic. Use only if it doesn't matter.
  pure def take(s: Set[a], n: int): Set[a] = {
    s.fold(Set(), (acc, x) => if (acc.size() < n) acc.union(Set(x)) else acc)
  }
}
