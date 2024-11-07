// -*- mode: Bluespec; -*-

module proofs {
  
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"

  type Leaf_t = {
    prefix: Term_t
  }

  type InnerOp = {
    prefix: Term_t,
    suffix: Term_t
  }

  // from https://github.com/cosmos/ics23/blob/master/proto/cosmos/ics23/v1/proofs.proto 

  type ExistenceProof = {
    key: BitArray,
    value: BitArray,
    leaf: Leaf_t,
    path: List[InnerOp],
  }

  type NonExistenceProof = {
    key: BitArray,
    left: ExistenceProof,
    right: ExistenceProof,
  }


  type CommitmentProof =
    | Exist(ExistenceProof) 
    | NonExist(NonExistenceProof)

  
  /// Return leaf with the largest key_hash smaller than k
  pure def leftNeighbor(t: TreeMap, k: BitArray): LeafNode =
    val smallerKeyNodes = t.values().filter(n => match n {
      | Leaf(l) => lessThan(l.key_hash, k)
      | Internal(_) => false
    }) 
    val someLeaf = smallerKeyNodes.fold({key_hash: [], value_hash: []}, (s, x) => 
     match x {
      | Leaf(l) =>  
            l
      | Internal(_) => s
      })
    smallerKeyNodes.fold( someLeaf, (s,x) =>
      match x {
      | Leaf(l) =>  
          if (lessThan(s.key_hash, l.key_hash) )
            l
          else 
            s
      | Internal(_) => s
      }
    )

  /// Return leaf with the smallest key_hash larger than k
  pure def rightNeighbor(t: TreeMap, k: BitArray): LeafNode =
    val largerKeyNodes = t.values().filter(n => match n {
      | Leaf(l) => lessThan(k, l.key_hash)
      | Internal(_) => false
    }) 
    val someLeaf = largerKeyNodes.fold({key_hash: [], value_hash: []}, (s, x) => 
     match x {
      | Leaf(l) =>  
            l
      | Internal(_) => s
      })
    largerKeyNodes.fold( someLeaf, (s,x) =>
      match x {
      | Leaf(l) =>  
          if (lessThan(l.key_hash, s.key_hash) )
            l
          else 
            s
      | Internal(_) => s
      }
    )

  pure def ics23_prove_existence(t: Tree, version: Version, key_hash: BitArray) : Option[List[InnerOp]] = 
    val prefixes_list = 0.to(key_hash.length()).map( i => key_hash.slice(0,i)).toList(listCompare)
    // fold over prefixes created from key_hash
    // i - index of current fold
    val result = prefixes_list.foldl({ path: List(), i: 0, found: false}, (result, key_prefix) => 
        val node = t.nodes.get({key_hash: key_prefix, version: version})
        match node {
          | Leaf(l) => { ...result, i: result.i+1, 
                                    found: l.key_hash == key_hash }
          | Internal(internal) => 
            val next_bit_0 = key_prefix.append(0)
            val innerOp = 
              if(prefixes_list[result.i+1] == next_bit_0) 
                { prefix: InternalNodeHashPrefix, 
                  suffix: match internal.right_child {
                            | None => Map()
                            | Some(c) => c.hash} }
              else 
                { prefix: InternalNodeHashPrefix.termConcat(match internal.left_child {
                                                              | None => Map()
                                                              | Some(c) => c.hash}), 
                  suffix: Map() }
            { path: result.path.append(innerOp),  
              i: result.i + 1, 
              found: false }
        }

    )
    if (result.found)
      Some(result.path.foldr(List(), (path_element, reversed_path ) => 
                                          reversed_path.append(path_element)))    
    else
      None

  // in implementation the key is passed instead of the key_hash
  pure def ics23_prove (t: Tree, key_hash: BitArray, version: Version): Option[CommitmentProof] =
    val state_storage_read = t.treeAtVersion(version)
                              .values()
                              .filter(n => match n {
                                      | Leaf(l) => l.key_hash == key_hash
                                      | Internal(_) => false
                              })
                              .map(n  => match n {
                                      | Leaf(l) => Some(l.value_hash)
                                      | Internal(_) => None})
                              .getOnlyElement()
    val proof = match state_storage_read {
      | Some(value) => // have key in database
        val p = ics23_prove_existence(t, version, key_hash)
        match p {
          | Some(path) =>
            Some(Exist( {  key: key_hash,
                    value: value,
                    leaf: {prefix: hashLeafNode({ key_hash: key_hash, value_hash: value})},
                    // not sure about the hash function. check with Rust
                    path: path }))
          | None => None 
        }
      | None => 
        val lneighbor = leftNeighbor(t.treeAtVersion(version), key_hash)
        val rneighbor = rightNeighbor(t.treeAtVersion(version), key_hash)
        val nep = { key: key_hash,
                    left: ics23_prove_existence(t, version, lneighbor.key_hash),
                    right: ics23_prove_existence(t, version, rneighbor.key_hash),}
        Some(NonExist(nep))
        // TODO: figure out what Rust is doing if left neighbor or right neigbor doesn't exist
        // Then we need to update the leftNeighbor and rightNeighbor functions accordingly
        // Then we need to figure out what goes into left and right of the non-existence proofs 
        // (I guess the type should be changed to Option[ExistenceProof])
    }
    proof



}