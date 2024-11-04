module proofs {
  
  import basicSpells.* from "./spells/basicSpells"
  import hashes.* from "./hashes"
  import tree.* from "./tree"
  export tree.*
  import node.* from "./node"
  import utils.* from "./utils"
  
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


}