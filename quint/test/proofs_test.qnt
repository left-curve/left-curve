module proofs_test {
  import proofs.* from "../proofs"
  import basicSpells.* from "../spells/basicSpells"
  import apply_fancy.* from "../apply_fancy"
  import rareSpells.* from "../spells/rareSpells"
  import node.* from "../node"
  import grug_ics23.* from "../grug_ics23"
  import utils.* from "../utils"

  val empty_tree: Tree = { nodes: Map(), orphans: Set()}
  
  run testA = {
      val ops =Set(
        { key_hash: [0, 0, 1, 1], op: Delete },
        { key_hash: [1, 0, 0, 0], op: Insert([5]) },
        { key_hash: [1, 1, 0, 0], op: Insert([12]) },
        { key_hash: [1, 1, 0, 1], op: Insert([8]) },
        { key_hash: [1, 1, 1, 0], op: Insert([7]) }
      )
      

      val t1 = empty_tree.apply(0,1,ops)
      val root = hash(t1.nodes.get({ key_hash: [], version: 1 }))
      val kh = [ 1, 1, 0, 1]
      val v = [8]

      val proof = ics23_prove(t1, kh, 1)

    val v1 = match proof.unwrap() {
                      | Exist(ep) => verifyMembership(root, ep, kh, v)
                      | NonExist(nep) =>  verifyNonMembership(root, nep, kh)
    }
  assert(v1 == true)
  }

  run testB = {
    val ops =Set(
        { key_hash: [0, 0, 1, 1], op: Delete },
        { key_hash: [1, 0, 0, 0], op: Insert([5]) },
        { key_hash: [1, 1, 0, 0], op: Insert([12]) },
        { key_hash: [1, 1, 0, 1], op: Insert([8]) },
        { key_hash: [1, 1, 1, 0], op: Insert([7]) }
      )
    val tree = empty_tree.apply(0,1,ops)

    tree.nodes.values().forall (nodes =>
        match nodes {
          | Internal => true
          | Leaf(n) =>
              val proof = ics23_prove(tree, n.key_hash, 1)
              val root = hash(tree.nodes.get({ key_hash: [], version: 1 }))
              match proof {
                | Some(p) =>
                  match p {
                    | Exist(ep) => verifyMembership(root, ep, n.key_hash, n.value_hash)
                    | NonExist(nep) => verifyNonMembership(root, nep, n.key_hash)
                  }
                | None => false}})
  }
  val testC = {
    val ops =Set(
        { key_hash: [0, 0, 1, 1], op: Delete },
        { key_hash: [1, 0, 0, 0], op: Insert([5]) },
        { key_hash: [1, 1, 0, 0], op: Insert([12]) },
        { key_hash: [1, 1, 0, 1], op: Insert([8]) },
        { key_hash: [1, 1, 1, 0], op: Insert([7]) }
      )
    val tree = empty_tree.apply(0,1,ops)
    val ops1 = Set(
      { key_hash: [0, 0, 1, 1], op: Insert([7]) },
      { key_hash: [0, 1, 0, 0], op: Insert([14]) },
      { key_hash: [0, 1, 1, 0], op: Insert([13]) },
      { key_hash: [1, 0, 1, 1], op: Delete },
      { key_hash: [1, 1, 0, 0], op: Delete }
    )
  val tree1 = tree.apply(1,2,ops1)
      // tree.nodes.values().forall (nodes =>
      //   match nodes {
      //     | Internal => true
      //     | Leaf(n) =>
      //         val proof = ics23_prove(tree, n.key_hash, 1)
      //         val root = hash(tree.nodes.get({ key_hash: [], version: 1 }))
      //         match proof {
      //           | Some(p) =>
      //             match p {
      //               | Exist(ep) => verifyMembership(root, ep, n.key_hash, n.value_hash)
      //               | NonExist(nep) => verifyNonMembership(root, nep, n.key_hash)
      //             }
      //           | None => false}})
      1.to(2).forall( version => 
      tree1.nodes.values().forall (nodes =>
        match nodes {
          | Internal => true
          | Leaf(n) =>
              val proof = ics23_prove(tree1, n.key_hash, version)
              val root = hash(tree1.nodes.get({ key_hash: [], version: version }))
              match proof {
                | Some(p) =>
                  match p {
                    | Exist(ep) => verifyMembership(root, ep, n.key_hash, n.value_hash)
                    | NonExist(nep) => verifyNonMembership(root, nep, n.key_hash)
                  }
                | None => false}}))
  // assert(true==true)
  }
}

