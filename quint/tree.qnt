// -*- mode: Bluespec; -*-

module tree {
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import node.* from "./node"
  export node.*
  import hashes.* from "./hashes"
  export hashes.*
  import utils.* from "./utils"
  
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
  pure def allLeafs(t: Tree) : Set[LeafNode] =
    val allNodes = t.nodes.keys().fold(Set(), (s, x) => s.union(Set(t.nodes.get(x))))
    allNodes.fold(Set(), (s,x) => match x {
                            | Internal(_) => s
                            | Leaf(x) => s.union(Set(x))
    })
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
      s.fold({version: 0 , key_hash: ROOT_BITS}, (longestOverlappingPrefix, node) => 
          if (node.key_hash.length() > longestOverlappingPrefix.key_hash.length() ) 
            node
          else 
            longestOverlappingPrefix
        )

  pure def createInternalOutOfLeafs(internalNodePrefix: List[int], existingLeaf: LeafNode, newLeaf: LeafNode) : InternalNode ={
    //getting first bit that does not overlap between new leaf and existing leaf
    // this dictates wether which leaf is going to be which
    val firstNonOverlappingBitOfExistingLeaf = existingLeaf
                                                .key_hash
                                                .slice(
                                                  internalNodePrefix.length(), 
                                                  internalNodePrefix.length() + 1)
    val newHash = {version: 0, hash: hashLeafNode(newLeaf)}
    val existingHash = {version: 0, hash: hashLeafNode(existingLeaf)}
        
    if(firstNonOverlappingBitOfExistingLeaf.head() == 1)
      {left_child: Some(newHash), right_child: Some(existingHash)}        
    else
      {left_child: Some(newHash), right_child: Some(existingHash)}
  }
  pure def getLastBitOfPrefix(prefix: List[int]) : int = 
    prefix.slice(prefix.length() - 1, prefix.length()).head()

  pure def addTwoLeaves(nodes: NodeId->Node, 
                        existingLeafPrefix: BitArray, existingLeaf: LeafNode, 
                        newLeafPrefix: BitArray, newLeaf: LeafNode): NodeId -> Node =
    nodes.put({version: 0, key_hash: existingLeafPrefix}, Leaf(existingLeaf))
           .put({version: 0, key_hash: newLeafPrefix}, Leaf(newLeaf))

  pure def newInternalNodes(t: Tree, existingLeafNodeId: NodeId, 
                            existingLeaf: LeafNode, newLeaf: LeafNode): NodeId -> Node =
      // get longest common prefix between already existing leaf and new leaf
      val longestCommonPrefixBetweenLeaves = commonPrefix(existingLeaf, newLeaf)
      val existingNodeNewPrefix = existingLeaf.key_hash.slice(0,longestCommonPrefixBetweenLeaves.length()+1)
      val newNodePrefix = newLeaf.key_hash.slice(0,longestCommonPrefixBetweenLeaves.length()+1)
      if(longestCommonPrefixBetweenLeaves.length() == 1){
        val internal = createInternalOutOfLeafs(longestCommonPrefixBetweenLeaves,existingLeaf, newLeaf)       
        t.nodes.put({version: 0, key_hash: longestCommonPrefixBetweenLeaves}, Internal(internal))
               .addTwoLeaves(existingNodeNewPrefix, existingLeaf, newNodePrefix, newLeaf)

      }else {
        val prefixCombos = 1.to(longestCommonPrefixBetweenLeaves.length())
                            .map(i => longestCommonPrefixBetweenLeaves.slice(0,i))
        val nodesToCreatePrefixList = prefixCombos.filter(prefix => 
                                                  prefix.length() >= existingLeafNodeId.key_hash.length())
                                                  .toList(listCompare)
        val lowestInternalNode = createInternalOutOfLeafs(longestCommonPrefixBetweenLeaves,existingLeaf, newLeaf)
        val updatedNodes = t.nodes.put({ version: 0, key_hash: longestCommonPrefixBetweenLeaves }, Internal(lowestInternalNode))
                                  .addTwoLeaves(existingNodeNewPrefix, existingLeaf, newNodePrefix, newLeaf)

        val foldOutcome = nodesToCreatePrefixList.foldl({
          nodes: updatedNodes,
          internalNodePrefix: longestCommonPrefixBetweenLeaves, 
          lastInternalNode: lowestInternalNode}, (iterator, el) => 
          val internalnode = if(getLastBitOfPrefix(iterator.internalNodePrefix)==1)
            { left_child:None, right_child: Some({ version: 0, hash: hashInternalNode(iterator.lastInternalNode) }) }
          else
            { left_child: Some({ version: 0, hash: hashInternalNode(iterator.lastInternalNode) }), right_child: None }
          
          { nodes: iterator.nodes.put({version:0, key_hash:el},Internal(internalnode)), 
            internalNodePrefix:el, 
            lastInternalNode:internalnode }
        )
        foldOutcome.nodes
      }
      

  pure def addLeafUnderInternal(t: Tree, internalToGrowLeaf: InternalNode, internalNodesNodeId: NodeId, 
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
      t.nodes.put(internalNodesNodeId, internal)
             .put(newPrefix(internalNodesNodeId, newLeaf), Leaf(newLeaf))

  pure def newPrefix(parent: NodeId, l: LeafNode): NodeId = 
      {version:0, key_hash: l.key_hash.slice(0,parent.key_hash.length()+1)}
  
  pure def getNodesOnPath(t: Tree, new_key_hash: Bytes_t) : Set[NodeId] = 
    val nodesOnPath = t.nodes.keys().filter( key => listPrefixCombinations(new_key_hash).contains(key.key_hash))
    val nodesToReturn = 
      if(nodesOnPath.size() == 1 and nodesOnPath == Set({ key_hash: ROOT_BITS, version: 0 }))
      val leaf = getLeafOrEmpty(t.nodes.get({ version:0,key_hash:[] }))
      val longestCommonPrefixBetweenKeys = commonPrefixBetweenKeys(leaf.key_hash, new_key_hash)
      val ids = 0.to(longestCommonPrefixBetweenKeys.length()).map(i => longestCommonPrefixBetweenKeys.slice(0,i))
      ids.fold(Set(), (nodeIds, id) => nodeIds.union(Set({version:0, key_hash: id})))
      else
        nodesOnPath
      
    nodesToReturn
  
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
    val nodesToUpdate: List[NodeId]= nodeIdsOnPath.toList(nodeCompare)
    nodesToUpdate.foldl(t.nodes, (nodes, nodeId) => 
      recomputeHashOnNode(nodes, nodeId))
  } 

  pure def constructStartTree(t: Tree): Tree = 
    val node = t.nodes.get({ version: 0, key_hash:ROOT_BITS })
    val leaf = match node {
      | Internal(_) => { key_hash:[], value_hash: [] }
      | Leaf(l) => l
    }
    val leafPrefix = leaf.key_hash.head()
    val internal  = if(leafPrefix == 0) 
      { left_child: Some({ version: 0, hash: hashLeafNode(leaf) }), right_child: None }
    else
      { left_child:None, right_child: Some({ version: 0, hash: hashLeafNode(leaf) }) }
    val nodes = t.nodes.put({ version: 0, key_hash: ROOT_BITS }, Internal(internal) )
                       .put({ version: 0, key_hash: List(leafPrefix) }, Leaf(leaf))
    { ...t, nodes: nodes }

  pure def leafHasNoOverlap(t: Tree, newLeaf: LeafNode) : bool = 
    allLeafs(t).filter(leaf=> 
                    leaf.key_hash.head() == newLeaf.key_hash.head()).size() == 0

  pure def isActualNode(nodes: NodeId -> Node, nodeId: NodeId) : bool =
    nodes.keys().contains(nodeId)

  pure def onlyOneElementInTree(t: Tree, existingLeaf: LeafNode,newLeaf: LeafNode ): Tree = 
    val updatedNodes: NodeId -> Node = if(newLeaf.key_hash.head() == 0) {
              val internalNode : InternalNode = {
                                  left_child: Some({ version: 0,
                                                hash: hashLeafNode(newLeaf)}), 
                                  right_child: Some({ 
                                                version: 0,
                                                hash: hashLeafNode(existingLeaf)})}
              t.nodes.put({version: 0, key_hash:ROOT_BITS}, Internal(internalNode))
                                        .put({version: 0, key_hash:[newLeaf.key_hash.head()]}, Leaf(newLeaf))
            }else{
              val internalNode : InternalNode = {
                                  left_child: Some({ version: 0,
                                                hash: hashLeafNode(existingLeaf)}), 
                                  right_child: Some({ version: 0,
                                                hash: hashLeafNode(newLeaf)})}
              t.nodes.put({version: 0, key_hash:ROOT_BITS}, Internal(internalNode))
                                        .put({version: 0, key_hash:[newLeaf.key_hash.head()]}, Leaf(newLeaf))
            }
        {...t, nodes: updatedNodes.put({version: 0, key_hash:[existingLeaf.key_hash.head()]}, Leaf(existingLeaf))}
  pure def insert(t: Tree, newLeaf: LeafNode): Tree = 
    if(t.nodes.keys().size() == 0){
      val n = makeNodeId(0, ROOT_BITS)
      val updatedNodes = t.nodes.put(n,Leaf(newLeaf))
      {...t, nodes: updatedNodes}
    }else if(false) t
    else{
      if(leafHasNoOverlap(t, newLeaf)){
        val rootNode = t.nodes.get({version:0, key_hash: ROOT_BITS})
        val tree = match rootNode{
          | Leaf(existingLeaf) => onlyOneElementInTree(t, existingLeaf, newLeaf)
          | Internal(rootNode) => {
            val updatedRootNode = 
              if(rootNode.left_child == None) {...rootNode, left_child: Some({version: 0, hash: hashLeafNode(newLeaf)})}
                else {...rootNode, right_child: Some({version: 0, hash: hashLeafNode(newLeaf)})}
            val newLeafNodeId = { version: 0, key_hash: List(newLeaf.key_hash.head())}
            val updatedNodes = t.nodes.put({version:0, key_hash: ROOT_BITS}, Internal(updatedRootNode))
                                      .put(newLeafNodeId, Leaf(newLeaf))
            {...t, nodes: updatedNodes}
          }
        }
        tree
      }else{
        val nodeIdsOnPath: Set[NodeId] = getNodesOnPath(t, newLeaf.key_hash)
        val nodeId: NodeId = longestPrefixNode(nodeIdsOnPath)
        val actualNode = isActualNode(t.nodes, nodeId)
        val tree = if(not(actualNode)) constructStartTree(t) else t  
        val nodeIdAfterReconstruction: NodeId = getNodesOnPath(tree, newLeaf.key_hash).longestPrefixNode()
        val nodeToChange = tree.nodes.get(nodeIdAfterReconstruction)
        val updatedNodes: NodeId -> Node = match nodeToChange {
            | Leaf(leafToChange) => newInternalNodes(tree, nodeIdAfterReconstruction, leafToChange, newLeaf)
            | Internal(internalToGrowLeaf) => addLeafUnderInternal(tree,internalToGrowLeaf, nodeIdAfterReconstruction, newLeaf)
        }
        val newNodesWithRecomputedHashes = recomputeHashesOnPath({...tree, nodes: updatedNodes}, nodeIdsOnPath)
          
        {...tree, nodes: newNodesWithRecomputedHashes}
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

}
