// -*- mode: Bluespec; -*-

module proofs {
  
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"

  type LeafOp = {
    prefix: Term_t
  }

  type InnerOp = {
    prefix: Term_t,
    suffix: Term_t
  }

  type ExistenceProof = {
    key: BitArray,
    value: BitArray,
    leaf: LeafOp,
    path: List[InnerOp],
  }

  type NonExistenceProof = {
    key: BitArray,
    left: Option[ExistenceProof],
    right: Option[ExistenceProof],
  }

  type CommitmentProof =
    | Exist(ExistenceProof) 
    | NonExist(NonExistenceProof)

  /// Returns optional list of InnerOps as a path to the leaf with particular key_hash
  pure def ics23_prove_existence(t: Tree, version: Version, key_hash: BitArray) 
  : Option[List[InnerOp]] =
    val prefixes_list = 0.to(key_hash.length()).map( i => key_hash.slice(0,i)).toList(listCompare)
    val r = prefixes_list.foldl({ path: List(), i: 0, found: false, child_version: version}, (iterator, key_prefix) => 
      if (iterator.found or not(t.nodes.keys().contains({key_hash: key_prefix, version: iterator.child_version}))) 
        iterator 
      else
        val node = t.nodes.get({key_hash: key_prefix, version: iterator.child_version})
        match node {
          | Leaf(l) => { ...iterator, i: iterator.i + 1, 
                                    found: l.key_hash == key_hash }
          | Internal(internal) => 
            val next_bit_0 = key_prefix.append(0)
            val child_version = if(prefixes_list[iterator.i + 1] == next_bit_0) 
                                  internal.left_child.unwrap().version
                                else 
                                  internal.right_child.unwrap().version
            val innerOp = 
              if(prefixes_list[iterator.i + 1] == next_bit_0) 
                { prefix: InternalNodeHashPrefix, 
                  suffix: match internal.right_child {
                            | None => Hash256_ZERO
                            | Some(c) => c.hash} }
              else 
                { prefix: InternalNodeHashPrefix
                            .termConcat(match internal.left_child {
                                          | None => Hash256_ZERO
                                          | Some(c) => c.hash}), 
                  suffix: Map() }

            { ...iterator, path: iterator.path.append(innerOp),  
              i: iterator.i + 1, child_version: child_version }
        }
    )
    if (r.found)
      Some(r.path.foldr(List(), (path_element, reversed_path ) => 
                                          reversed_path.append(path_element)))    
    else
      None

  /// Return leaf with the largest key_hash smaller than k
  pure def leftNeighbor(t: TreeMap, k: BitArray): Option[LeafNode] =
    val smallerKeyNodes = t.values().filter(n => match n {
      | Leaf(l) => lessThan(l.key_hash, k)
      | Internal(_) => false
    }) 
    if(smallerKeyNodes.empty()) None else 
      val someLeaf = smallerKeyNodes.fold({key_hash: [], value_hash: []}, (s, x) => 
        match x {
          | Leaf(l) =>  
                l
          | Internal(_) => s
          })
      Some(smallerKeyNodes.fold( someLeaf, (s,x) =>
        match x {
          | Leaf(l) =>  
              if (lessThan(s.key_hash, l.key_hash) )
                l
              else 
                s
          | Internal(_) => s
        }
      ))

  /// Return leaf with the smallest key_hash larger than k
  pure def rightNeighbor(t: TreeMap, k: BitArray): Option[LeafNode] =
    val largerKeyNodes = t.values().filter(n => match n {
      | Leaf(l) => lessThan(k, l.key_hash)
      | Internal(_) => false
    }) 
    if(largerKeyNodes.empty()) None else 
      val someLeaf = largerKeyNodes.fold({key_hash: [], value_hash: []}, (s, x) => 
        match x {
          | Leaf(l) =>  
                l
          | Internal(_) => s
          })
      Some(largerKeyNodes.fold( someLeaf, (s,x) =>
        match x {
          | Leaf(l) =>  
              if (lessThan(l.key_hash, s.key_hash) )
                l
              else 
                s
          | Internal(_) => s
        }
      ))

  /// Returns optional CommitmentProof based for the given key_hash. 
  /// In implementation the key is passed instead of the key_hash.
  pure def ics23_prove (t: Tree, key_hash: BitArray, version: Version): Option[CommitmentProof] =
    val optionalValueForKey = t.treeAtVersion(version)
                            .allLeafs()
                            .filter(l => l.key_hash == key_hash)
                            .map( l => Some(l.value_hash))
    val state_storage_read = if(optionalValueForKey.empty()) None else optionalValueForKey.getOnlyElement()
    match state_storage_read {
      | Some(value) =>
        val p = ics23_prove_existence(t, version, key_hash)
        match p {
          | Some(path) =>
              Some(Exist(
                { key: key_hash,
                  value: value,
                  leaf: { prefix: LeafNodeHashPrefix },
                  path: path }
              ))
          | None => None
        }
      | None =>
        val lneighborOption: Option[LeafNode] = leftNeighbor(t.treeAtVersion(version), key_hash)
        val leftNeighborExistenceProof: Option[ExistenceProof] = match lneighborOption {
          | Some(lneighbor) => 
            val pathOption = ics23_prove_existence(t, version, lneighbor.key_hash)
            match pathOption {
              | Some(path) => Some({
                key: lneighbor.key_hash,
                value: lneighbor.value_hash,
                leaf: { prefix: LeafNodeHashPrefix },
                path: path
              })
              | None => None
            }
          | None => None
        }
        val rneighborOption: Option[LeafNode]  = rightNeighbor(t.treeAtVersion(version), key_hash)
        val rightNeighborExistenceProof: Option[ExistenceProof] = match rneighborOption {
          | Some(rneighbor) => 
            val pathOption = ics23_prove_existence(t,version, rneighbor.key_hash)
            match pathOption {
              | Some(path) => Some({
                key: rneighbor.key_hash,
                value: rneighbor.value_hash,
                leaf: { prefix: LeafNodeHashPrefix },
                path: path
              })
              | None => None
            }
          | None => None
        }
        val nep : NonExistenceProof = { key: key_hash,
                                        left: leftNeighborExistenceProof,
                                        right: rightNeighborExistenceProof }
        Some(NonExist(nep))
    }
}
