module proofs {
  
  import basicSpells.* from "./spells/basicSpells"
  import rareSpells.* from "./spells/rareSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"

  type InnerOp = {
    prefix: Term_t,
    suffix: Term_t
  }
  
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

  pure def ics23_prove_existence(t: Tree, version: Version, key_hash: BitArray) : List[InnerOp] = 
    val prefixes_list = 0.to(key_hash.length()).map( i => key_hash.slice(0,i)).toList(listCompare)
    // fold over prefixes created from key_hash
    // i - index of current fold
    val path = prefixes_list.foldl({ path: List(), i: 0, found: false}, (result, key_prefix) => 
        val node = t.nodes.get({key_hash: key_prefix, version: version})
        match node {
          | Internal(internal) => 
            val next_bit_0 = key_prefix.append(0)
            val innerOp = if(prefixes_list[result.i+1] == next_bit_0) 
              { prefix: InternalNodeHashPrefix, suffix: match internal.right_child {
              | None => Map()
              | Some(c) => c.hash
              } }
            else { 
              prefix: InternalNodeHashPrefix.termConcat(
                match internal.left_child {
                  | None => Map()
                  | Some(c) => c.hash
                }), 
              suffix: Map() }
            {path: result.path.append(innerOp),  i: result.i+1, found: false }
          | Leaf(l) => { ...result, i: result.i+1, found: l.key_hash == key_hash }
        }

    )
    path.path.foldr(List(), (path_element, reversed_path ) => reversed_path.append(path_element))    
}