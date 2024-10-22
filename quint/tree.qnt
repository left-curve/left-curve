// -*- mode: Bluespec; -*-

module tree {
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import node.* from "./node"
  export node.*
  import hashes.* from "./hashes"
  export hashes.*

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
  //TODO: rename key_hash to key_hash_prefix
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

  pure def makeOrphanId(osv: Version, v: Version, kh: List[int]): OrphanId = 
    { orphaned_since_version: osv, version: v, key_hash: kh }

  pure def makeNodeId(v: Version, kh: BitArray): NodeId = 
    { version: v, key_hash: kh }

  pure def mark_node_as_orphaned(tree: Tree, orphaned_since_version: Version, version: Version, key_hash: BitArray): Tree = {
    pure val orphan = {
      orphaned_since_version: orphaned_since_version, 
      version: version, 
      key_hash: key_hash,
    }
    { ...tree, orphans: tree.orphans.union(Set(orphan)) }
  }

  pure def isNodeOrphaned(nodeId: NodeId, orphans: Set[OrphanId]): bool = {
    orphans.exists(el => (el.version == nodeId.version and el.key_hash == nodeId.key_hash))
  }

  pure def prune(tree: Tree, up_to_version: Version): Tree =  {
    pure val orphans_to_be_removed = tree.orphans.filter(orphan => 
      orphan.orphaned_since_version <= up_to_version
    )
    pure val prunned_orphans = tree.orphans.filter(orphan => 
      orphan.orphaned_since_version > up_to_version
    )

    pure val keys_of_non_orphanes = tree.nodes.keys().filter(nodeId => 
      not(isNodeOrphaned(nodeId, orphans_to_be_removed))
    )
    pure val updated_nodes = keys_of_non_orphanes.mapBy(x => tree.nodes.get(x))
    { nodes: updated_nodes, orphans: prunned_orphans }
  }

  var tree : Tree

  action init =
    tree' = {
        orphans: Set(),
        nodes: Map(),
    }
}
