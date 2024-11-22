// -*- mode: Bluespec; -*-

module proofs_test {
  import proofs.* from "../proofs"
  import basicSpells.* from "../spells/basicSpells"
  import apply_super_simple.* from "../apply_super_simple"
  import rareSpells.* from "../spells/rareSpells"
  import node.* from "../node"
  import grug_ics23.* from "../grug_ics23"
  import utils.* from "../utils"
  import apply_state_machine.* from "../apply_state_machine"
  
  val empty_tree: Tree = { nodes: Map(), orphans: Set()}
  
  pure def onlyDeletes(ops) = ops.filter( x => x.op != Delete).empty()
  pure def getOnlyInserts(ops) = ops.filter(x => x.op != Delete)
  
  run twoDifferentTreesTest = 
    val tA = empty_tree
    val tB = empty_tree

    nondet key_hashes_as_maps_tree_a = all_key_hashes_as_maps.oneOf()
    nondet key_hashes_as_maps_tree_b = all_key_hashes_as_maps.oneOf()
    and {
      key_hashes_as_maps_tree_a.empty() == false,
      key_hashes_as_maps_tree_b.empty() == false,
      key_hashes_as_maps_tree_a != key_hashes_as_maps_tree_b
    } implies 
      nondet kms_with_value_tree_a = key_hashes_as_maps_tree_a
                                        .setOfMaps(VALUES)
                                        .oneOf()
      nondet kms_with_value_tree_b = key_hashes_as_maps_tree_b
                                        .setOfMaps(VALUES)
                                        .oneOf()
                                  
      pure val opsA = kms_with_value_tree_a.to_operations()
      pure val opsB = kms_with_value_tree_b.to_operations()
      
      pure val tAUpdated = tA.apply(0, 1, opsA)
      pure val tBUpdated = tB.apply(0, 1, opsB)
      and {
        tAUpdated.nodes != Map(),
        tBUpdated.nodes != Map(),
        tAUpdated.nodes.allLeafs()
                        .map(x => x.key_hash)
                        .exclude(tBUpdated.nodes.allLeafs()
                                                .map(x => x.key_hash))
                        .empty() == false,
      } implies
        nondet key_non_existant_in_b = tAUpdated.nodes.allLeafs()
                                                      .map(x => x.key_hash)
                                                      .exclude(tBUpdated.nodes.allLeafs().map(x => x.key_hash))
                                        .oneOf()
        val value = tAUpdated.nodes.allLeafs()
                      .filter(x=>x.key_hash == key_non_existant_in_b)
                      .getOnlyElement()
                      .value_hash
        val proof = ics23_prove(tAUpdated, key_non_existant_in_b, 1)

        val rootB_hash = hash(tBUpdated.nodes.get({ key_hash: ROOT_BITS, version: 1 }))

        val verifiedProofOnB =  match proof {
                    | Some(p) =>
                      match p {
                        | Exist(ep) =>  verifyMembership(rootB_hash, ep, key_non_existant_in_b, value) 
                        | NonExist(nep) =>  true             
                      }
                    | None => true
          }
        debug("reached assert", assert(verifiedProofOnB == false))

  run twoDifferentTreesByOnlyValuesTest = 
    val tA = empty_tree
    val tB = empty_tree

    nondet key_hashes_as_maps_tree_a = all_key_hashes_as_maps.oneOf()
    
    key_hashes_as_maps_tree_a.empty() == false implies 
      nondet kms_with_value_tree_a = key_hashes_as_maps_tree_a.setOfMaps(VALUES).oneOf()
      pure val opsA = kms_with_value_tree_a.to_operations()
      opsA.onlyDeletes() == false implies 
        pure val opsB = opsA.fold(Set(), (updatedValsInOps, opA) => 
                                    updatedValsInOps
                                      .union(Set(
                                        { key_hash: opA.key_hash, 
                                          op: match opA.op {
                                              | Delete => opA.op
                                              | Insert(value) => Insert([value[0] + 1])
                                          }
                                        })))
        val tAUpdated = tA.apply(0, 1, opsA)
        val tBUpdated = tB.apply(0, 1, opsB)

        nondet leaf = tAUpdated.nodes.allLeafs().oneOf()
        val proof = ics23_prove(tAUpdated, leaf.key_hash, 1)
        val rootB_hash = hash(tBUpdated.nodes.get({ key_hash: ROOT_BITS, version: 1 }))

        val verifiedProofOnB =  match proof {
                    | Some(p) =>
                      match p {
                        | Exist(ep) =>  verifyMembership(rootB_hash, ep, leaf.key_hash, leaf.value_hash) 
                        | NonExist(nep) =>  true         
                      }
                    | None => true
          }
         debug("reached assert", assert(verifiedProofOnB == false))

  
  run twoSameTreesDifferentByOneValueTest = 
    val tA = empty_tree
    val tB = empty_tree

    nondet key_hashes_as_maps = all_key_hashes_as_maps.oneOf()
    key_hashes_as_maps.empty() == false implies 
      nondet kms_with_value = key_hashes_as_maps.setOfMaps(VALUES).oneOf()

      pure val opsA = kms_with_value.to_operations()
      opsA.onlyDeletes() == false implies
        nondet opToChangeValue = opsA.filter(op => op.op != Delete).oneOf()
        val unchangedValue = match opToChangeValue.op { 
          | Insert(v) => v
          | Delete => [-1]
        }
        val opsBUpdated = opsA.fold(Set(), (ops, op) => 
                                    if(op != opToChangeValue) ops.union(Set(op)) 
                                    else ops.union(Set({ ...op, op: Insert([222]) })))
        val tAUpdated = tA.apply(0, 1, opsA)
        val tBUpdated = tB.apply(0, 1, opsBUpdated)

        nondet leaf = tAUpdated.nodes.allLeafs().oneOf()
        val proof = ics23_prove(tAUpdated, leaf.key_hash, 1)
        val rootB_hash = hash(tBUpdated.nodes.get({ key_hash: ROOT_BITS, version: 1 }))

        val verifiedProofOnB =  match proof {
                    | Some(p) =>
                      match p {
                        | Exist(ep) =>  verifyMembership(rootB_hash, ep, leaf.key_hash, unchangedValue)
                        | NonExist(nep) =>  true
                      }
                    | None => true
          }
        debug("reached assert", assert(verifiedProofOnB == false))
    
  
  run twoDifferentTreesSameByOnlyOneKVTest = 
    val tA = empty_tree
    val tB = empty_tree

    nondet key_hashes_as_maps_tree_a = all_key_hashes_as_maps.oneOf()
    nondet key_hashes_as_maps_tree_b = all_key_hashes_as_maps.oneOf()
    and {
      key_hashes_as_maps_tree_a.empty() == false,
      key_hashes_as_maps_tree_b.empty() == false,
      key_hashes_as_maps_tree_a != key_hashes_as_maps_tree_b
    } implies
      nondet kms_with_value_tree_a = key_hashes_as_maps_tree_a
                                        .setOfMaps(VALUES)
                                        .oneOf()
      nondet kms_with_value_tree_b = key_hashes_as_maps_tree_b
                                        .setOfMaps(VALUES)
                                        .oneOf()
                                  
      pure val opsA = kms_with_value_tree_a.to_operations()
      pure val opsB = kms_with_value_tree_b.to_operations()
      and {
        opsA.onlyDeletes() == false,
        opsB.onlyDeletes() == false,
      } implies 
        nondet opsAValueToUseInB = opsA.filter(x => x.op == Delete).oneOf()
        nondet opsBValueToUpdated = opsB.filter(x => x.op == Delete).oneOf()
        val updatedOpsB = opsB.fold(Set(), (ops, op) => if(op == opsBValueToUpdated)
                                                        ops.union(Set(opsAValueToUseInB))
                                                        else ops.union(Set(op)))
        pure val tAUpdated = tA.apply(0, 1, opsA)
        pure val tBUpdated = tB.apply(0, 1, updatedOpsB)
        
        val leaf = { key_hash: opsAValueToUseInB.key_hash, value_hash: match opsAValueToUseInB.op{
          | Delete => [-1]
          | Insert(v) => v
        }}
        val proof = ics23_prove(tAUpdated, leaf.key_hash, 1)
        val rootB_hash = hash(tBUpdated.nodes.get({ key_hash: ROOT_BITS, version: 1 }))

        val verifiedProofOnB =  match proof {
                    | Some(p) =>
                      match p {
                        | Exist(ep) =>  verifyMembership(rootB_hash, ep, leaf.key_hash, leaf.value_hash)
                        | NonExist(nep) =>  true
                      }
                    | None => true
          }
        debug("reached assert", assert(true==true))


  run verificationOnPrunnedTreeTest =
    val t = empty_tree
    nondet key_hashes_as_maps_tree_0 = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value_tree_0 = key_hashes_as_maps_tree_0
                                        .setOfMaps(VALUES)
                                        .oneOf()

    nondet key_hashes_as_maps_tree_1 = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value_tree_1 = key_hashes_as_maps_tree_1
                                        .setOfMaps(VALUES)
                                        .oneOf()

    nondet key_hashes_as_maps_tree_2 = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value_tree_2 = key_hashes_as_maps_tree_2
                                        .setOfMaps(VALUES)
                                        .oneOf()
    
    nondet key_hashes_as_maps_tree_3 = all_key_hashes_as_maps.oneOf()
    nondet kms_with_value_tree_3 = key_hashes_as_maps_tree_3
                                        .setOfMaps(VALUES)
                                        .oneOf()
    and {
      key_hashes_as_maps_tree_0.empty() == false,
      key_hashes_as_maps_tree_1.empty() == false,
      key_hashes_as_maps_tree_2.empty() == false,
      key_hashes_as_maps_tree_3.empty() == false,
    } implies
      val ops = Set(kms_with_value_tree_0.to_operations(), 
                    kms_with_value_tree_1.to_operations(), 
                    kms_with_value_tree_2.to_operations(), 
                    kms_with_value_tree_3.to_operations())

      val ret =  ops.fold( { tree: t, version: 0 }, (iter, batch) => 
        {tree: iter.tree.apply(iter.version, iter.version + 1, batch), version: iter.version + 1})
      nondet leafAtActiveVersion = ret.tree.treeAtVersion(ret.version).allLeafs().oneOf()
      val proofAtActiveVersion = ics23_prove(ret.tree, leafAtActiveVersion.key_hash, ret.version)

      nondet versionToPrune = 2.to(ret.version-1).oneOf()
      val prunnedTree = ret.tree.prune(versionToPrune)
      val proofAtPrunnedTree = ics23_prove(prunnedTree, leafAtActiveVersion.key_hash, ret.version)
      
      val prunedRootHash = hash(prunnedTree.nodes.get({key_hash: ROOT_BITS, version: ret.version}))
      val unprunedRootHash = hash(ret.tree.nodes.get({key_hash: ROOT_BITS, version: ret.version}))
      
      val proofVerifiedOnPrunedRoot = match proofAtActiveVersion {
        | Some(p) =>
          match p {
            | Exist(ep) =>  verifyMembership(prunedRootHash, ep, leafAtActiveVersion.key_hash, leafAtActiveVersion.value_hash)
            | NonExist(nep) =>  true
          }
        | None => true
      }
      val proofFromPrunedTreeVerifiedOnUnprunned = match proofAtPrunnedTree {
        | Some(p) =>
          match p {
            | Exist(ep) =>  verifyMembership(unprunedRootHash, ep, leafAtActiveVersion.key_hash, leafAtActiveVersion.value_hash)
            | NonExist(nep) =>  true
          }
        | None => true
      }
      debug("reached assert", assert( all{ 
        proofVerifiedOnPrunedRoot == true,
        proofFromPrunedTreeVerifiedOnUnprunned == true
      }))


  run leafNotExistsThenExistsTest = 
    val t0 = empty_tree
    nondet key_hashes_as_maps_tree = all_key_hashes_as_maps.oneOf()
    key_hashes_as_maps_tree.empty() == false implies 
      nondet kms_with_value_tree = key_hashes_as_maps_tree
                                          .setOfMaps(VALUES)
                                          .oneOf()
      val ops = kms_with_value_tree.to_operations()
      ops.onlyDeletes() == false implies
        nondet opToInsert = ops.oneOf()
        and{
          opToInsert.op != Delete,
          ops.exclude(Set(opToInsert)).onlyDeletes() == false
        } implies 
          val firstBatch = ops.exclude(Set(opToInsert))
          val secondBatch = Set(opToInsert)

          val t1 = t0.apply(0,1,firstBatch)
          val nonexistenceProof = ics23_prove(t1, opToInsert.key_hash, 1)
          val hashedRootAfterFirstBatch = hash(t1.nodes.get({ key_hash: ROOT_BITS, version: 1}))
          val proofFromTreeAfterFirstBatchVerified = match nonexistenceProof {
            | Some(p) =>
              match p {
                | Exist(ep) =>  false
                | NonExist(nep) =>  verifyNonMembership(hashedRootAfterFirstBatch, nep, 
                                                      opToInsert.key_hash)
              }
            | None => false
          }
          
          val t2 = t1.apply(1,2,secondBatch)
          val hashedRootAfterSecondBatch = hash(t2.nodes.get({ key_hash: ROOT_BITS, version: 2}))
          val proofFromTreeAfterFirstBatchShouldNotBeVerified = match nonexistenceProof {
            | Some(p) =>
              match p {
                | Exist(ep) =>  true
                | NonExist(nep) =>  verifyNonMembership(hashedRootAfterSecondBatch, nep, 
                                                      opToInsert.key_hash)
              }
            | None => true
          }
          debug("reached assert", assert(
            all{
              t1 != t2,
              hashedRootAfterSecondBatch != hashedRootAfterFirstBatch,
              proofFromTreeAfterFirstBatchVerified == true,
              proofFromTreeAfterFirstBatchShouldNotBeVerified == false
            }))
  
  run leafExistsThenNotExistsTest = 
    val t0 = empty_tree
    nondet key_hashes_as_maps_tree = all_key_hashes_as_maps.oneOf()
    key_hashes_as_maps_tree.empty() == false implies 
      nondet kms_with_value_tree = key_hashes_as_maps_tree
                                          .setOfMaps(VALUES)
                                          .oneOf()
      val ops = kms_with_value_tree.to_operations()
      and{
        ops.onlyDeletes() == false,
        ops.getOnlyInserts().size() > 2
      } implies
          val firstBatch = ops
          val t1 = t0.apply(0,1,firstBatch)
          nondet randomLeafToDelete = t1.nodes.allLeafs().oneOf()

          val existenceProof = ics23_prove(t1, randomLeafToDelete.key_hash, 1)
          val hashedRootAfterFirstBatch = hash(t1.nodes.get({ key_hash: ROOT_BITS, version: 1}))
          
          val proofFromTreeAfterFirstBatchVerified = match existenceProof {
            | Some(p) =>
              match p {
                | Exist(ep) =>  verifyMembership(hashedRootAfterFirstBatch, ep, 
                                                      randomLeafToDelete.key_hash, 
                                                      randomLeafToDelete.value_hash)
                | NonExist(nep) =>  false
              }
            | None => false
          }
          val secondBatch = Set({ key_hash:randomLeafToDelete.key_hash, op: Delete })
          val t2 = t1.apply(1, 2, secondBatch)
          val hashedRootAfterSecondBatch = hash(t2.nodes.get({ key_hash: ROOT_BITS, version: 2}))
          val proofFromTreeAfterFirstBatchShouldNotBeVerified = match existenceProof {
            | Some(p) =>
              match p {
                | Exist(ep) =>  verifyMembership(hashedRootAfterSecondBatch, ep, 
                                                      randomLeafToDelete.key_hash, 
                                                      randomLeafToDelete.value_hash)
                | NonExist(nep) =>  true
              }
            | None => true
          }
          debug("reached assert", assert(
            all{
              t1 != t2,
              hashedRootAfterSecondBatch != hashedRootAfterFirstBatch,
              proofFromTreeAfterFirstBatchVerified == true,
              proofFromTreeAfterFirstBatchShouldNotBeVerified == false
            }))
}
