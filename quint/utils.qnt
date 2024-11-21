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

  pure def safeGet(m: a -> b, k: a): Option[b] = {
    if (m.has(k)) Some(m.get(k)) else None
  }

  pure def filterMap(s: Set[a], f: (a) => Option[b]): Set[b] = {
    s.fold(Set(), (acc, e) => {
      match f(e) {
        | Some(x) => acc.union(Set(x))
        | None => acc
      }
    })
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

  pure def mapToTuples(m: a -> b): Set[(a, b)] = {
    m.keys().map(k => (k, m.get(k)))
  }

  pure def max_height(values_with_key_hash) = {
    values_with_key_hash.fold(0, (max_height, key) => max(max_height, key.key_hash.length()))
  }

  pure def add_nodes(nodes: NodeId -> Node, new_nodes: Set[(NodeId, Node)]): NodeId -> Node = {
    new_nodes.fold(nodes, (nodes, new_node) => nodes.put(new_node._1, new_node._2))
  }

  pure def into_child(version: Version, outcome: Outcome): Option[Child] = {
    match outcome {
      | Updated(node) => Some({
        version: version,
        hash: node.hash(),
      })
      | Unchanged(_) => None // Unreachable if _ is not None
      | Deleted => None // Unreachable
    }
  }

  pure def is_updated_or_deleted(outcome: Outcome): bool = {
    match outcome {
      | Updated(_) => true
      | Deleted => true
      | _ => false
    }
  }

  pure def is_unchanged(outcome: Outcome): bool = {
    match outcome {
      | Unchanged(_) => true
      | _ => false
    }
  }

  pure def unchanged_leaf(outcome: Outcome): bool = {
    match outcome {
      | Unchanged(optional) => match optional {
        | Some(node) => match node {
          | Leaf(_) => true
          | _ => false
        }
        | _ => false
      }
      | _ => false
    }
  }

  pure def updated_to_leaf(outcome: Outcome): bool = {
    match outcome {
      | Updated(node) => match node {
        | Leaf(_) => true
        | _ => false
      }
      | _ => false
    }
  }
}
