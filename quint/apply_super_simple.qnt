// -*- mode: Bluespec; -*-

module apply_super_simple {
  import basicSpells.* from "./spells/basicSpells"
  import node.* from "./node"
  import tree.* from "./tree"
  import hashes.* from "./hashes"
  import utils.* from "./utils"

  pure def apply(tree: Tree, old_version: Version, new_version: Version, batch: Set[OperationOnKey]): Tree = {
    batch.fold(tree, (t, op) => {
      match op.op {
        | Insert(value_hash) => {
          pure val newLeaf = { key_hash: op.key_hash, value_hash: value_hash }
          super_simple_apply(t, old_version, new_version, newLeaf, true)
        }
        | Delete => {
          pure val leafToDelete = { key_hash: op.key_hash, value_hash: [] }
          super_simple_apply(t, old_version, new_version, leafToDelete, false)
        }
      }
    })
  }
  pure def extractOrphansFromNewVersion(oldTree: Tree, updatedTree: Tree, old_version: Version, new_version: Version) : Set[OrphanId] = 
    oldTree.orphans
        .union(updatedTree.treeAtVersion(old_version)
              .keys()
              .fold(Set(), (orphans, key) =>
                    if(updatedTree.treeAtVersion(new_version)
                        .keys()
                        .contains(key))
                      orphans
                    else                       
                      orphans.union(
                      Set({ orphaned_since_version: new_version, 
                          version: key.version, 
                          key_hash: key.key_hash})
                      )
                    )
                  )

  pure def unpackChild(child: Option[Child]) : Child = 
    pure val EMPTY_MAP = Map()
    match child {
      | Some(c) => { version: c.version, hash: c.hash }
      | None => { version: -1, hash: EMPTY_MAP }
    }
  pure def unpackInternalNode(internal: InternalNode) : { left: Child, right: Child } =
    { left: internal.left_child.unpackChild(), 
      right: internal.right_child.unpackChild() }

  pure def cmpInternalAndNode(internal : InternalNode, internalOldVersion: InternalNode) : bool = 
    pure val unpackedNew = internal.unpackInternalNode()
    pure val unpackedOld = internalOldVersion.unpackInternalNode()
    unpackedNew.left.hash == unpackedOld.left.hash and unpackedNew.right.hash == unpackedOld.right.hash
  
  pure def updateChildVersions(nodes: NodeId->Node, new_version: Version) : NodeId->Node = 
    val internalNodes = allInternals(nodes)
    
    internalNodes.keys().fold(nodes, (nodes, key) =>
        if (new_version > key.version and internalNodes.has({...key, version: new_version}))
          val internalOld = internalNodes.get(key)
          val internalNew = internalNodes.get({ ...key, version: new_version })
          val unpackedOld = internalOld.unpackInternalNode()
          val unpackedNew = internalNew.unpackInternalNode()
          // At most one children will have the same hash because, if they both had same hashes
          // we would have removed the new_node in `removeDuplicateEntries`
          if( unpackedNew.left.hash == unpackedOld.left.hash and  unpackedNew.left.version != unpackedOld.left.version )
            nodes.set({ ...key, version: new_version }, Internal({ ...internalNew, left_child: Some( unpackedOld.left ) }))
          else if ( unpackedNew.right.hash == unpackedOld.right.hash and unpackedNew.right.version != unpackedOld.right.version )
            nodes.set({ ...key, version: new_version }, Internal({ ...internalNew, right_child: Some( unpackedOld.right) }))
          else nodes
        else 
        nodes
      )
  pure def allPrefixesForKey (l: List[a]): Set[List[a]] = 
    0.to(l.length()).map(i => l.slice(0,i))

  pure def removeDuplicateEntries(nodes: NodeId->Node, nodeKeyHash: BitArray, new_version: Version, isInsert: bool) : NodeId->Node = 
    if(isInsert) nodes.keys().fold(nodes, (nodes, key) => 
      if(new_version > key.version and nodes.keys().contains({...key, version: new_version}))
        val nodeForKey = nodes.get(key)
        val nodeForKeyWithNewVersion = nodes.get({...key, version: new_version})
        if(nodeForKey == nodeForKeyWithNewVersion)
          nodes.mapRemove({...key, version: new_version})
        else
            match nodeForKeyWithNewVersion {
             | Internal(internalNewVersion) 
                => match nodeForKey {
                  | Internal(internalOldVersion) =>  
                    if(cmpInternalAndNode(internalNewVersion, internalOldVersion) and not(key.isRoot()))  
                      nodes.mapRemove({...key, version: new_version})
                    else nodes
                  | Leaf(_) => nodes
                }
             | Leaf(_) => nodes
           }
      else 
        nodes
    )
    else 
      val keyPrefixes = allPrefixesForKey(nodeKeyHash)
      
      nodes.keys().fold(nodes, (nodes, key) => 
        if(new_version > key.version and nodes.keys().contains({...key, version: new_version}))
          val nodeForKey = nodes.get(key)
          val nodeForKeyWithNewVersion = nodes.get({...key, version: new_version})
          if(nodeForKey == nodeForKeyWithNewVersion and not(keyPrefixes.contains(key.key_hash)))
            nodes.mapRemove({...key, version: new_version})
          else
              match nodeForKeyWithNewVersion {
              | Internal(internalNewVersion) 
                  => match nodeForKey {
                    | Internal(internalOldVersion) =>  
                      if(cmpInternalAndNode(internalNewVersion, internalOldVersion) 
                          and not(key.isRoot()
                          and not(keyPrefixes.contains(key.key_hash))))  
                        nodes.mapRemove({...key, version: new_version})
                      else nodes
                    | Leaf(_) => nodes
                  }
              | Leaf(_) => nodes
            }
        else 
          nodes
      )

  pure def super_simple_apply(tree: Tree, old_version: Version, new_version: Version, insert: LeafNode, isInsert: bool): Tree = {
    val treeForOldVersion = treeAtVersion(tree, old_version)
    pure val existing_keys_with_values = treeForOldVersion.keys().fold(Set(), (acc, k) => {
        match tree.nodes.get(k) {
          | Leaf(leaf) => acc.union(Set(leaf))
          | _ => acc
        }
    })
  
    pure val keys_with_values = if(isInsert) existing_keys_with_values
                                              .filter(x => x.key_hash != insert.key_hash)
                                              .union(Set(insert))
                                else existing_keys_with_values
                                              .filter(x => x.key_hash != insert.key_hash)
    pure val result = range(0, MAX_HASH_LENGTH).foldr((Set(), keys_with_values.map(kv => { key_hash: kv.key_hash, node: Leaf(kv) })), (i, acc) => {
      acc._2.fold(acc, (acc, kv) => {
        pure val prefix = kv.key_hash.slice(0, i)
        pure val left = acc._2.find(k => k.key_hash == prefix.append(0))
        pure val right = acc._2.find(k => k.key_hash == prefix.append(1))
        pure val queue_1 = acc._2.filter(k => k.key_hash != prefix.append(0) and k.key_hash != prefix.append(1))
        pure val queue_2 = if (left != None and right != None) {
          pure val internal_node = Internal({
            left_child: Some({ version: new_version, hash: left.unwrap().node.hash() }),
            right_child: Some({ version: new_version, hash: right.unwrap().node.hash() })
          })
          // old queue, don't pop
          acc._2.union(Set({ key_hash: prefix, node: internal_node }))
        } else if (left != None) {
          match left.unwrap().node {
            | Internal(_) => {
               pure val internal_node = Internal({
                 left_child: Some({ version: new_version, hash: left.unwrap().node.hash() }),
                 right_child: None,
               })
               // old queue, don't pop
               acc._2.union(Set({ key_hash: prefix, node: internal_node }))
            }
            | Leaf(leaf) => queue_1.union(Set({ key_hash: prefix, node: left.unwrap().node }))
          }
        } else if (right != None) {
          match right.unwrap().node {
            | Internal(_) => {
               pure val internal_node = Internal({
                 left_child: None,
                 right_child: Some({ version: new_version, hash: right.unwrap().node.hash() })
               })
               // old queue, don't pop
               acc._2.union(Set({ key_hash: prefix, node: internal_node }))
            }
            | Leaf(leaf) => queue_1.union(Set({ key_hash: prefix, node: right.unwrap().node }))
          }
        } else if (i == 0) {
          pure val root = queue_1.getOnlyElement()
          acc._2.union(Set({ key_hash: prefix, node: root.node }))
        } else {
          acc._2
        }
        (acc._1.union(queue_2), queue_2)
      })
    })
    
    val nodes = result._2.map(kv => ({version: new_version, key_hash: kv.key_hash}, kv.node))
                      .fold(tree.nodes, (nodes, kv) => nodes.put(kv._1, kv._2))
                      .removeDuplicateEntries(insert.key_hash, new_version, isInsert)
                      .updateChildVersions(new_version)
    
    val orphans = extractOrphansFromNewVersion(tree, { nodes: nodes, orphans: Set() },
                                               old_version, new_version)
      
    { nodes: nodes, orphans: orphans }
  }
}
