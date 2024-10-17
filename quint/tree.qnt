// -*- mode: Bluespec; -*-

module tree {
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import node.* from "./node"
  import hashes.* from "./hashes"

  type Outcome =
      | Unchanged(Option[Node])
      | Updated(Node)
      | Deleted

  type Operation =
      | Insert(Bytes_t)
      | Delete

  type OperationOnBytes = { bytes: Bytes_t, op: Operation }

  type BitArray = List[int]
  type Version = int
  type NodeId = {version: Version, key_hash: BitArray}
  type OrphanId = {
      orphaned_since_version: Version,
      version: Version,
      key_hash: BitArray
  }

  type Tree = {
      nodes : NodeId -> Node,
      orphans: Set[OrphanId]
  }

  pure val ROOT_BITS: BitArray = []

  pure def makeOrphanId(osv: Version, v: Version, kh: List[int]): OrphanId = { orphaned_since_version: osv, version: v, key_hash: kh}
  pure def makeNodeId(v: Version, kh: BitArray) : NodeId = {version: v, key_hash: kh}

  pure def mark_node_as_orphaned(tree: Tree, orphaned_since_version: Version, version: Version, key_hash: BitArray) : Tree = {
      pure val orphan = { orphaned_since_version: orphaned_since_version, version: version, key_hash: key_hash }
      { ...tree, orphans: tree.orphans.union(Set(orphan)) }
  }

  pure def prune(orphans: Set[OrphanId], up_to_version: Version) : Set[OrphanId] =  {
      orphans.filter(orphan => orphan.orphaned_since_version > up_to_version)
  }

  var tree : Tree

  action init =
    tree' = {
        orphans: Set(),
        nodes: Map(),
    }
}
