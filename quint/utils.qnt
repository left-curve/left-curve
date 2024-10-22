module utils {
  import basicSpells.* from "./spells/basicSpells"
  import node.* from "./node"
  import hashes.* from "./hashes"
  val keylength = 4

  pure def unwrap(value: Option[a]): a = {
    match value {
      | None => Map().get(value)
      | Some(x) => x
    }
  }
  pure def getOnlyElement(s: Set[a]): a = {
    pure val hack = s.map(e => ("value", e)).setToMap()
    if (s.size() != 1) {
      hack.get("error: expected singleton")
    } else {
      hack.get("value")
    }
  }
  pure def commonPrefix(a: LeafNode, b: LeafNode) : Bytes_t =
    val indList = range(1, keylength + 1)
    indList.foldl([], (s, x) => if (a.key_hash.slice(0, x) == b.key_hash.slice(0, x)) 
                                    b.key_hash.slice(0, x) 
                                else s )
}