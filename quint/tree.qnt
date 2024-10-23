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

  pure def listPrefixCombinations(new_key_hash: BitArray) : Set[BitArray]  =
       0.to(new_key_hash.length()).map(i => new_key_hash.slice(0,i))

  pure def longestPrefixNode(s: Set[NodeId]) : NodeId =
      s.fold({version: 0 , key_hash: []}, (longestOverlappingPrefix, node) => 
          if (node.key_hash.length() > longestOverlappingPrefix.key_hash.length() ) 
            node
          else 
            longestOverlappingPrefix
        )

  pure def newInternalNode(existingLeaf: LeafNode, newLeaf: LeafNode): InternalNode =
      // TODO: recompute parent hashes, etc.
      // get common prefix between already existing leaf and new leaf
      val internalNodePrefix = commonPrefix(existingLeaf, newLeaf)
      //getting first bit that does not overlap between new leaf and existing leaf
      // this dictates wether which leaf is going to be which
      val firstNonOverlappingBitOfExistingLeaf = existingLeaf
                                                  .key_hash
                                                  .slice(
                                                    internalNodePrefix.length(), 
                                                    internalNodePrefix.length() + 1)
      val newHash = {version: 0, hash: hashLeafNode(newLeaf)}
      val existingHash = {version: 0, hash: hashLeafNode(existingLeaf)}
      
      if(firstNonOverlappingBitOfExistingLeaf.head() == 1){
        // existing leaf should be right child
        // new leaf is left child
        {left_child: Some(newHash), right_child: Some(existingHash)}        
      }
      else{
        // existing leaf should be left child
        // new leaf is right child
        {left_child: Some(newHash), right_child: Some(existingHash)}
      }

  pure def addLeafUnderInternal(internalToGrowLeaf: InternalNode, internalNodesNodeId: NodeId, 
                                  newLeaf: LeafNode): InternalNode =
      // TODO: recompute parent hashes, etc.
      // figure out whether l is left or right, and puts the hash(l) there
      val firstNonOverlappingBitOfNewLeaf = newLeaf
                                              .key_hash
                                              .slice(
                                                internalNodesNodeId.key_hash.length(), 
                                                internalNodesNodeId.key_hash.length() + 1)
      if(firstNonOverlappingBitOfNewLeaf.head() == 1)
        {...internalToGrowLeaf, right_child: Some({version: 0, hash: hashLeafNode(newLeaf)})}
      else
        {...internalToGrowLeaf, left_child: Some({version: 0, hash: hashLeafNode(newLeaf)})}

  pure def newPrefix(parent: NodeId, l: LeafNode): NodeId = 
      {version:0, key_hash: l.key_hash.slice(0,parent.key_hash.length()+1)}
  
  pure def getNodesOnPath(t: Tree, new_key_hash: Bytes_t) : Set[NodeId] = 
    t.nodes.keys().filter( key => listPrefixCombinations(new_key_hash).contains(key.key_hash))
  
  pure def recomputeHashesOnPath(t: Tree, nodeIdsOnPath: Set[NodeId], newLeaf: LeafNode) : Tree = {
    t
  }
  pure def leafHasNoOverlap(t: Tree, newLeaf: LeafNode) : bool = 
    not(t.nodes.keys()
        .contains({version: 0, 
                  key_hash: List(newLeaf.key_hash.head())
                  }))

  pure def insert(t: Tree, newLeaf: LeafNode): Tree = 
    if(t.nodes.keys().size() == 0){
      val key_hash_prefix : Bytes_t = List(newLeaf.key_hash.head())
      val n = makeNodeId(0, key_hash_prefix)
      val updatedNodes = t.nodes.put(n,Leaf(newLeaf))
      {...t, nodes: updatedNodes}
    }else{
      if(leafHasNoOverlap(t, newLeaf)){
        if(newLeaf.key_hash.head() == 0) {
          val existingLeaf = t.nodes.get({version:0, key_hash: [1]})
          val internalNode : InternalNode = {
                              left_child: Some({ version: 0,
                                            hash: hashLeafNode(newLeaf)}), 
                              right_child: Some({ 
                                            version: 0,
                                            hash: hash(existingLeaf)})}
          val updatedNodes = t.nodes.put({version: 0, key_hash:[]}, Internal(internalNode))
                                    .put({version: 0, key_hash:[newLeaf.key_hash.head()]}, Leaf(newLeaf))
          {...t, nodes: updatedNodes}      
        }else{
          val existingLeaf = t.nodes.get({version:0, key_hash: [0]})
          val internalNode : InternalNode = {
                              left_child: Some({ version: 0,
                                            hash: hash(existingLeaf)}), 
                              right_child: Some({ version: 0,
                                            hash: hashLeafNode(newLeaf)})}
          val updatedNodes = t.nodes.put({version: 0, key_hash:[]}, Internal(internalNode))
                                    .put({version: 0, key_hash:[newLeaf.key_hash.head()]}, Leaf(newLeaf))
          {...t, nodes: updatedNodes}
        }

      }else{
        val nodeIdsOnPath: Set[NodeId] = getNodesOnPath(t, newLeaf.key_hash)
        val nodeId: NodeId = longestPrefixNode(nodeIdsOnPath)
        val nodeToChange = t.nodes.get(nodeId)
        val replacement: InternalNode = match nodeToChange {
            | Leaf(leafToChange) => newInternalNode(leafToChange, newLeaf)
            | Internal(internalToGrowLeaf) => addLeafUnderInternal(internalToGrowLeaf, nodeId, newLeaf)
        }
        val updatedNodes = t.nodes
                            .set(nodeId, Internal(replacement))
                            .put(newPrefix(nodeId, newLeaf), Leaf(newLeaf))

        val newUpdatedNodes = match nodeToChange {
            | Leaf(leaf) => updatedNodes.put(newPrefix(nodeId,leaf), Leaf(leaf))  
            | Internal(internal) => updatedNodes  
        }
        // TODO: recompute parent hashes; use nodeIdOnPath to iterate in descending length()
        val newNodesWithRecomputedHashes = recomputeHashesOnPath(t, nodeIdsOnPath, newLeaf)

        {...t, nodes: newUpdatedNodes}
      }
    }
      
      // TODO: add special cases of empty tree, and only one leaf
      // TODO: delete function + update
      // TODO: implement helper functions


  var tree: Tree

  action init =
    tree' = {
        orphans: Set(),
        nodes: Map()
  }

  action insertAction(l) =
      tree' = insert(tree,l)

  run Example = {
    val leafs = List({key_hash: [1, 1, 1, 1], value_hash: [1]}, {key_hash: [0, 0, 0, 0], value_hash: [7]})
    // TODO non-deterministic initialization
    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }
  
}
