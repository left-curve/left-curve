// -*- mode: Bluespec; -*-

module compl {

  import grug_ics23.* from "./grug_ics23"
  import tree.* from "./tree"
  import proofs.* from "./proofs"
  import basicSpells.* from "./spells/basicSpells"
  import apply_state_machine.* from "./apply_state_machine"

  /// For each node in the version of the tree, ics23_prove returns
  /// an existence proof, and the proof can be verified against the root
  /// hash
  pure def complMemProof (tree: Tree, v: Version): bool =
    val t = treeAtVersion(tree, v)
    t.values().forall(node =>
      match node {
        | Leaf(n) =>
          match ics23_prove(tree, n.key_hash, v) {
            | Some(cp) => match cp {
                | Exist(ep) => 
                  val root = hash(tree.nodes.get({ key_hash: ROOT_BITS, version: v }))
                  verifyMembership(root, ep, n.key_hash, n.value_hash)
                | NonExist(_) => false
              }
            | None => false
          }
        | Internal(_) => true
      }
    )

  val complMemProofInv =
    1.to(version).forall(v => complMemProof(tree, v))


  //TODO: this only works for 4. Not sure how to generalize
  pure def allKeyHashes = 
    tuples(Set(0,1), Set(0,1), Set(0,1), Set(0,1)).fold(Set(), (s,x) =>
      s.union(Set(List(x._1, x._2, x._3, x._4))))
  
  /// For each key_hash that is not in the version of the tree, 
  /// ics23_prove returns a non-existence proof, and the proof 
  /// can be verified against the root hash
  pure def complNonMemProof (tree: Tree, v: Version): bool =
    val t = treeAtVersion(tree, v)
    val allKeys = allKeyHashes
    val allContainedKeys = t.values().fold(Set(), (s,x) =>
      match x {
        | Leaf(n) => s.union(Set(n.key_hash))
        | Internal(_) => s
      })
    allKeys.exclude(allContainedKeys).forall(k =>
      match ics23_prove(tree, k, v) {
            | Some(cp) => match cp {
                | NonExist(nep) => 
                  val root = hash(tree.nodes.get({ key_hash: ROOT_BITS, version: v }))
                  verifyNonMembership(root, nep, k)
                | Exist(_) => false
              }
            | None => false
      }
    )

  val complNonMemProofInv =
    1.to(version).forall(v => complNonMemProof(tree, v))

}