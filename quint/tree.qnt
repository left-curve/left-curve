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
//      original tree
//        root
//        / \
//       0   1
//          
//    tree after inserts
//        root
//        / \
//       0   1
//          / \
//      null   11
//            /  \
//         null  111
//               /  \
//            1110  1111
// >>> newLeaf1
// { key_hash: [1, 1, 1, 1], value_hash: [5] }
// >>> newLeaf2
// { key_hash: [1, 1, 1, 0], value_hash: [42] }
  pure def getLastBitOfPrefix(prefix: List[int]) : int = 
    prefix.slice(prefix.length() - 1, prefix.length()).head()
  pure def newInternalNodes(t: Tree, existingLeaf: LeafNode, newLeaf: LeafNode): NodeId -> Node =
      // get longest common prefix between already existing leaf and new leaf
      val longestCommonPrefixBetweenLeaves = commonPrefix(existingLeaf, newLeaf)
      
      if(longestCommonPrefixBetweenLeaves.length() == 1){
        val internalNodePrefix = commonPrefix(existingLeaf, newLeaf)
        val nodesMap : NodeId -> Node = Map()
        //getting first bit that does not overlap between new leaf and existing leaf
        // this dictates wether which leaf is going to be which
        val firstNonOverlappingBitOfExistingLeaf = existingLeaf
                                                    .key_hash
                                                    .slice(
                                                      internalNodePrefix.length(), 
                                                      internalNodePrefix.length() + 1)
        val newHash = {version: 0, hash: hashLeafNode(newLeaf)}
        val existingHash = {version: 0, hash: hashLeafNode(existingLeaf)}
        
        val internal = if(firstNonOverlappingBitOfExistingLeaf.head() == 1){
          {left_child: Some(newHash), right_child: Some(existingHash)}        
        }
        else{
          {left_child: Some(newHash), right_child: Some(existingHash)}
        }
        val m = Map()
        m.put({version:0, key_hash: internalNodePrefix}, Internal(internal))
      }else {
        val prefixCombos = 1.to(longestCommonPrefixBetweenLeaves.length()).map(i => longestCommonPrefixBetweenLeaves.slice(0,i))
        val orderPrefixCombos = prefixCombos.toList(listCompare)
        // orderPrefixCombos((Set(), ))
        // val 
        // val test = orderPrefixCombos.foldl((Set(),getLastBitOfPrefix(orderPrefixCombos[0])) , (internalNodesToGrow, prefix) => )
        val m: NodeId->Node = Map()
        m
      }
      

  pure def addLeafUnderInternal(internalToGrowLeaf: InternalNode, internalNodesNodeId: NodeId, 
                                  newLeaf: LeafNode): NodeId -> Node =
      // figure out whether l is left or right, and puts the hash(l) there
      val firstNonOverlappingBitOfNewLeaf = newLeaf
                                              .key_hash
                                              .slice(
                                                internalNodesNodeId.key_hash.length(), 
                                                internalNodesNodeId.key_hash.length() + 1)
      val internal = if(firstNonOverlappingBitOfNewLeaf.head() == 1)
        Internal({...internalToGrowLeaf, right_child: Some({version: 0, hash: hashLeafNode(newLeaf)})})
      else
        Internal({...internalToGrowLeaf, left_child: Some({version: 0, hash: hashLeafNode(newLeaf)})})
      val m = Map()
      m.put(internalNodesNodeId, internal)

  pure def newPrefix(parent: NodeId, l: LeafNode): NodeId = 
      {version:0, key_hash: l.key_hash.slice(0,parent.key_hash.length()+1)}
  
  pure def getNodesOnPath(t: Tree, new_key_hash: Bytes_t) : Set[NodeId] = 
    t.nodes.keys().filter( key => listPrefixCombinations(new_key_hash).contains(key.key_hash))
  
/// input a tree where the hashes of the node with nodeId might be wrong
  pure def recomputeHashOnNode(t: NodeId -> Node, nodeId: NodeId) : NodeId -> Node = {
      val newNode = match t.get(nodeId) {
            | Internal(_) =>
                            val leftChildPrefix = nodeId.key_hash.append(0)
                            val hashLeftChild = if(t.keys().contains({version: 0, key_hash: leftChildPrefix}))
                                                Some({version: 0, hash: hash(t.get({version: 0, key_hash: leftChildPrefix}))})                                                
                                                else None
                            
                            val rightChildPrefix = nodeId.key_hash.append(1)
                            val hashRightChild = if(t.keys().contains({version: 0, key_hash: rightChildPrefix}))
                                                Some({version: 0, hash: hash(t.get({version: 0, key_hash: rightChildPrefix}))})                                                
                                                else None
              Internal({left_child: hashLeftChild, right_child: hashRightChild})
            | Leaf(l) => Leaf(l)
      }
      t.set(nodeId, newNode)
  }

  pure def recomputeHashesOnPath(t: Tree, nodeIdsOnPath: Set[NodeId]) : NodeId -> Node = {
    //.exclude(Set(longestPrefixNode(nodeIdsOnPath)))
    val nodesToUpdate: List[NodeId]= nodeIdsOnPath.toList(nodeCompare)
    nodesToUpdate.foldl(t.nodes, (nodes, nodeId) => 
      recomputeHashOnNode(nodes, nodeId))
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
        val replacements: NodeId -> Node = match nodeToChange {
            | Leaf(leafToChange) => newInternalNodes(t, leafToChange, newLeaf)
            | Internal(internalToGrowLeaf) => addLeafUnderInternal(internalToGrowLeaf, nodeId, newLeaf)
        }
        val updatedNodes = replacements.keys().fold(t.nodes, (nodes, key) => nodes.put(nodeId, replacements.get(key)))
        
        val updatedNodes1 = updatedNodes
                            .put(newPrefix(nodeId, newLeaf), Leaf(newLeaf))

        val newUpdatedNodes = match nodeToChange {
            | Leaf(leaf) => updatedNodes1.put(newPrefix(nodeId,leaf), Leaf(leaf))  
            | Internal(internal) => updatedNodes1  
        }
        // TODO: recompute parent hashes; use nodeIdOnPath to iterate in descending length()
        val newNodesWithRecomputedHashes = recomputeHashesOnPath({...t, nodes: newUpdatedNodes}, nodeIdsOnPath)
        
        {...t, nodes: newNodesWithRecomputedHashes}
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
    // [1]
    // [1,0] None
    // [1,1]
    // [1,1,0]
    // [1,1,1]
    val leafs = List({key_hash: [1, 1, 1, 1], value_hash: [5]}, {key_hash: [0, 0, 0, 0], value_hash: [7]}, {key_hash: [1, 0, 0, 1], value_hash: [42]})
    // TODO non-deterministic initialization
    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }
  val newLeaf1 = {key_hash: [1, 0, 1, 0, 1], value_hash: [5]}
  val newLeaf2 = {key_hash: [1, 0, 1, 0, 0], value_hash: [42]}

  run ExampleSC = {
    val leafs = List({key_hash: [1, 1, 1, 1], value_hash: [5]}, {key_hash: [0, 0, 0, 0], value_hash: [7]}, {key_hash: [1, 1, 0, 1], value_hash: [42]})
    // TODO non-deterministic initialization
    init
    .then(leafs.length().reps(i => insertAction(leafs[i])))
    .expect(true) //some invariant
  }
}
