// -*- mode: Bluespec; -*-

module tree {
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import node.* from "./node"
  export node.*
  import hashes.* from "./hashes"
  export hashes.*
	import utils.* from"./utils"
	export utils.*
	
	pure val MAX_HASH_LENGTH = 4

  type Outcome =
      | Unchanged(Option[Node])
      | Updated(Node)
      | Deleted

  type Operation =
      | Insert(Bytes_t)
      | Delete

  // For types like -> batch: Vec<(Hash256, Op<Hash256>)>,
  type OperationOnKey = { key_hash: BitArray, op: Operation }
  // For types like -> batch: Vec<(Hash256, Hash256)>
  type KeyWithValue = { key_hash: BitArray, value_hash: Bytes_t }

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

  pure def makeOrphanId(osv: Version, v: Version, kh: List[int]): OrphanId = { orphaned_since_version: osv, version: v, key_hash: kh}
  pure def makeNodeId(v: Version, kh: BitArray) : NodeId = {version: v, key_hash: kh}

  pure def mark_node_as_orphaned(tree: Tree, orphaned_since_version: Version, version: Version, key_hash: BitArray) : Tree = {
      pure val orphan = { orphaned_since_version: orphaned_since_version, version: version, key_hash: key_hash }
      { ...tree, orphans: tree.orphans.union(Set(orphan)) }
  }
  pure def isNodeOrphaned(nodeId: NodeId, orphans: Set[OrphanId]) : bool = {
    orphans.exists(el => (el.version == nodeId.version and el.key_hash == nodeId.key_hash))
  }
  pure def prune(tree: Tree, up_to_version: Version) : Tree =  {
      val orphans_to_be_removed = tree.orphans.filter(orphan => orphan.orphaned_since_version <= up_to_version)
      val prunned_orphans = tree.orphans.filter(orphan => orphan.orphaned_since_version > up_to_version)

      val keys_of_non_orphanes = tree.nodes.keys().filter(nodeId => isNodeOrphaned(nodeId, orphans_to_be_removed) == false)
      val updated_nodes = keys_of_non_orphanes.mapBy( x => tree.nodes.get(x) )

      {nodes: updated_nodes, orphans: prunned_orphans}
  }
	pure def getKeyWithLongestOverlappingPrefix(tree: Tree, keyValue: KeyWithValue) : NodeId = {
    // get all prefix combination from keyValue.key_hash
    // e.g. 
    //    key = [0, 1, 1, 0]
    //    prefixCombos = [], [0], [0,1], [0,1,1], [0,1,1,0]
    val prefixCombos : Set[List[int]] = 0.to(MAX_HASH_LENGTH).map(i => keyValue.key_hash.slice(0,i))
    // get all keys that are contained in the prefix combinations
    // e.g. 
    //  First example: 
    //    prefix combinations = [], [0], [0,1], [0,1,1], [0,1,1,0]
    //    keys in tree: [0], [1]
    //    keys that contain some prefix = [0]
    //  Second example: 
    //    prefix combinations = [], [0], [0,1], [0,1,1], [0,1,1,0]
    //    keys in tree: [0], [1], [0,0], [0,1], [0,0,0], [0,1,1]
    //    keys that contain some prefix = [0], [0,1], [0,1,1]
    val prefixComboContainingKeys  = tree.nodes.keys().filter( key => prefixCombos.contains(key.key_hash))
    // get longest prefix that overlaps with the key
    // e.g. 
    //    keys that contain some prefix = [0], [0,1], [0,1,1]
    //    longestOverlappingPrefix = 3
    val longestOverlappingPrefix = prefixComboContainingKeys.fold(0, (maxLen, y) => if (y.key_hash.length() > maxLen ) { y.key_hash.length()} else {maxLen})
    // find prefix whose size is equal to longest overlap between prefix and key
    // e.g. 
    //    longestOverlappingPrefix = 3
    //    keys that contain some prefix = [0], [0,1], [0,1,1]
    //    perfectPrefixOverlapCombo = [0,1,1]
    val perfectPrefixOverlapCombo = prefixComboContainingKeys.filter(key => key.key_hash.length() == longestOverlappingPrefix)
    // since perfectPrefixOverlapCombo is a set, this line pops that perfect element from perfectPrefixOverlapCombo
    // e.g.
    //  perfectPrefixOverlapCombo = Set({version: 0, [0,1,1]})
    //  out = {version: 0, [0,1,1]}
    perfectPrefixOverlapCombo.getOnlyElement()
  }

  var tree: Tree

  action init =
    tree' = {
        orphans: Set(),
        nodes: Map(makeNodeId(0, ROOT_BITS) -> Internal({left_child: None, right_child: None}))
  }
}
