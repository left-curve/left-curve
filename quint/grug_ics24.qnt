// -*- mode: Bluespec; -*-

// This is a protocol specification of ICS23, tuned towards the Grug JMT
// (The original spec was for the IAVL case.)
//
// For details of ICS23, see:
// https://github.com/cosmos/ibc/tree/main/spec/core/ics-023-vector-commitments
//
// For the implementation of ICS23, see:
// https://github.com/cosmos/ics23
//
// We still have to parameterize the spec with the data structure parameters
// such as MinPrefixLen, MaxPrefixLen, ChildSize, and hash.
//
// Igor Konnov, Informal Systems, 2022-2023
// Josef Widder, Informal Systems, 2024
// Aleksandar Ignjatijevic, Informal Systems, 2024

module grug_ics23 {
  import hashes.* from "./hashes"
  import node.* from "./node"
  import basicSpells.* from "./spells/basicSpells"
  import utils.* from "./utils"
  // type aliases for readability
  type Key_t = Bytes_t
  type Value_t = Bytes_t
  type CommitmentRoot_t = Term_t
  type CommitmentProof_t = Term_t

  type Ics23InnerSpec = {
    MinPrefixLen: int, 
    MaxPrefixLen: int,
    ChildSize: int,
    EmptyChild: Term_t
  }

  pure val Ics23ProofSpec: Ics23InnerSpec= {
    MinPrefixLen: 1,
    MaxPrefixLen: 1,
    ChildSize: 32,
    EmptyChild: Hash256_ZERO
  }

  type LEAF_T = {
    prefix: Term_t
  }

  type INNER_T = {
    prefix: Term_t,
    suffix: Term_t
  }

  /// a proof of existence of (key, value)
  type ExistsProof_t = {
    key: Key_t, 
    value: Value_t, 
    leaf: LEAF_T, 
    path: List[INNER_T]
  }

  /// a proof of non-existence of a key
  type NonExistsProof_t = {
    key: Key_t, 
    left: Option[ExistsProof_t], 
    right: Option[ExistsProof_t]
  }

  /// VerifyMembership returns true iff
  /// proof is an ExistenceProof for the given key and value AND
  /// calculating the root for the ExistenceProof matches
  /// the provided CommitmentRoot
  def verifyMembership(root: CommitmentRoot_t,
      proof: ExistsProof_t, key: Key_t, value: Value_t): bool = {
    // TODO: specify Decompress
    // TODO: specify the case of CommitmentProof_Batch
    // TODO: CheckAgainstSpec ensures that the proof can be verified
    //       by the spec checker
    verify(proof, root, key, value)
  }

  /// verify that a proof matches a root
  def verify(proof, root, key, value) = and {
    key == proof.key,
    value == proof.value,
    root == existsCalculate(proof)
  }
  /// calculate a hash from an exists proof
  def existsCalculate(p: ExistsProof_t): CommitmentProof_t = 
    val leafHash = hashLeafNode({ key_hash: p.key, value_hash: p.value})    
    // the inner node nodeHashes are concatenated and hashed upwards
    p.path.foldl(leafHash,
      (child, inner) =>
        termHash(inner.prefix.termConcat(child).termConcat(inner.suffix)))

}
