// -*- mode: Bluespec; -*-

module sound {

  import grug_ics23.* from "./grug_ics23"
  import tree.* from "./tree"
  import proofs.* from "./proofs"
  import basicSpells.* from "./spells/basicSpells"
  import apply_state_machine.* from "./apply_state_machine"

  //TODO: this only works for 4. Not sure how to generalize
  pure def allKeyHashes = 
    tuples(Set(0,1), Set(0,1), Set(0,1), Set(0,1)).fold(Set(), (s,x) =>
      s.union(Set(List(x._1, x._2, x._3, x._4))))

  /// For each keyhash (among all possible ones) where, ics23_prove returns
  /// an existence proof, the key is actually in the tree
  pure def soundMemProof (tree: Tree, v: Version): bool =
    allKeyHashes.forall(k =>
      match ics23_prove(tree, k, v) {
            | Some(cp) => match cp {
                | Exist(ep) => 
                    treeAtVersion(tree, v).values().filter(n => match n {
                      | Leaf(n) => n.key_hash == k
                      | Internal(_) => false
                    }).size() > 0
                | NonExist(_) => true
              }
            | None => true
      }
    )

  
  val soundMemProofInv =
    1.to(version).forall(v => soundMemProof(tree, v))



  
  /// For each keyhash (among all possible ones) where, ics23_prove returns
  /// an non-existence proof, the key is not in the tree
  pure def soundNonMemProof (tree: Tree, v: Version): bool =
    allKeyHashes.forall(k =>
      match ics23_prove(tree, k, v) {
            | Some(cp) => match cp {
                | NonExist(nep) => 
                    treeAtVersion(tree, v).values().filter(n => match n {
                      | Leaf(n) => n.key_hash == k
                      | Internal(_) => false
                    }).size() == 0
                | Exist(_) => true
              }
            | None => true
      }
    )

  val soundNonMemProofInv =
    1.to(version).forall(v => soundNonMemProof(tree, v))

}