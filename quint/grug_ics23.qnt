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

  // ICS23 IavlSpec has:
  // MinPrefixLength = 4
  // MaxPrefixLength = 12
  // ChildSize = 33 // 32 bytes in SHA256 + 1 byte for the length marker
  // !!! UPDATE !!!
  // ICS23 specs are placed in grug/jellyfish-merkle/src/ics23.rs
  // https://github.com/left-curve/left-curve/blob/7a0451dfad14d607722c33bec20ae56dd5c3bffa/grug/jellyfish-merkle/src/ics23.rs#L18-L37
  // In Left Curve Grug JellyFish Merkle Tree the following parameters are set to 1
  pure val MinPrefixLen = 1
  pure val MaxPrefixLen = 1
  // It is crucial to make sure that ChildSize > MaxPrefixLen
  // !!! UPDATE!!! 
  //  this parameter is set to 32 because that is the size of SHA256 
  //  and that value is used in configuring ics23 verification spec
  //  in LeftCurve Grug JellyFish Merkle Tree implementation
  pure val ChildSize = 32 //32 bytes in SHA256


  // Empty child is a predefined sequence that fills an absent child.
  // It's defined in InnerSpec and it has the value of Hash256::ZERO
  // which is: Hash(0000000000000000000000000000000000000000000000000000000000000000)
  pure val EmptyChild = Hash256_ZERO

  type LEAF_T = {
    // The implementation additionally stores hashing and length functions:
    // hash, prehashKey, prehashValue, len. Since we fix the spec to IAVL,
    // we do not have to carry them around.
    // !!! UPDATE !!! 
    // The same applies for Left Curve Grug JellyFish Merkle Tree
    prefix: Term_t
  }

  type INNER_T = {
    // The implementation additionally stores the hashing function.
    // Since we fix the spec to IAVL, we do not have to carry it around.
    // !!! UPDATE !!!
    // The same applies for Left Curve Grug JellyFish Merkle Tree
    prefix: Term_t,
    suffix: Term_t
  }

  /// a proof of existence of (key, value)
  type ExistsProof_t = {
    key: Key_t, value: Value_t, leaf: LEAF_T, path: List[INNER_T]
  }

  /// a proof of non-existence of a key
  type NonExistsProof_t = {
    key: Key_t, left: Option[ExistsProof_t], right: Option[ExistsProof_t]
  }

  /// calculate a hash from an exists proof
  def existsCalculate(p: ExistsProof_t): CommitmentProof_t = 
    // using specific hashLeafNode for Grug JMT
    val leafHash = hashLeafNode({ key_hash: p.key, value_hash: p.value})    
    // the inner node nodeHashes are concatenated and hashed upwards
    p.path.foldl(leafHash,
      (child, inner) =>
        termHash(inner.prefix.termConcat(child).termConcat(inner.suffix)))

  /// verify that a proof matches a root
  def verify(proof, root, key, value) = and {
    key == proof.key,
    value == proof.value,
    root == existsCalculate(proof)
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
  /// checks if an op has the expected padding
  def hasPadding(inner: INNER_T,
      minPrefixLen: int, maxPrefixLen: int, suffixLen: int): bool = and {
    termLen(inner.prefix) >= minPrefixLen,
    termLen(inner.prefix) <= maxPrefixLen,
    // When inner turns left, suffixLen == ChildSize,
    // that is, we store the hash of the right child in the suffix.
    // When inner turns right, suffixLen == 0,
    // that is, we store the hash of the left child in the prefix.
   termLen( inner.suffix) == suffixLen
  }
    def hasPadding1(inner: INNER_T,
      minPrefixLen: int, maxPrefixLen: int, suffixLen: int): bool = and {
    termLen(inner.prefix) >= minPrefixLen,
    termLen(inner.prefix) <= maxPrefixLen,
    // When inner turns left, suffixLen == ChildSize,
    // that is, we store the hash of the right child in the suffix.
    // When inner turns right, suffixLen == 0,
    // that is, we store the hash of the left child in the prefix.
   termLen( inner.suffix) == suffixLen
  }
  /// This will look at the proof and determine which order it is...
  /// So we can see if it is branch 0, 1, 2 etc... to determine neighbors
  /// https://github.com/confio/ics23/blob/a4daeb4c24ce1be827829c0841446abc690c4f11/go/proof.go#L400-L411
  def orderFromPadding(inner: INNER_T): (int, bool) = {
    // Specialize orderFromPadding to IavlSpec:
    // ChildOrder = { 0, 1 }
    // branch = 0: minp, maxp, suffix = MinPrefixLen, MaxPrefixLen, ChildSize
    // branch = 1: minp, maxp, suffix =
    //             ChildSize + MinPrefixLen, ChildSize + MaxPrefixLen, 0
    // !!! UPDATE !!!
    // The same applies for Left Curve Grug JellyFish Merkle Tree
    if (hasPadding(inner, MinPrefixLen, MaxPrefixLen, ChildSize)) {
      // the node turns left
      (0, true)
    } else if (hasPadding(inner, ChildSize + MinPrefixLen,
                          ChildSize + MaxPrefixLen, 0)) {
      // the node turns right
      (1, true)
    } else {
      // error
      (0, false)
    }
  }

  /// leftBranchesAreEmpty returns true if the padding bytes correspond to all
  /// empty siblings on the left side of a branch, ie. it's a valid placeholder
  /// on a leftmost path
  def leftBranchesAreEmpty(inner: INNER_T): bool = and {
    // the case of leftBranches == 0 returns false
    val order = orderFromPadding(inner)
    order._2 and order._1 != 0,
    // the remaining case is leftBranches == 1, see orderFromPadding
    // actualPrefix = len(inner.prefix) - 33
    // !!! UPDATE !!!
    // For Left Curve Grug we have changed ChildSize to accustom their JellyFish Merkle Tree, 
    // therefore previous comment is outdated
    // actualPrefix = len(inner.prefix) - 32
    termLen(inner.prefix) >= ChildSize,
    // !!!!!!! IMPLEMENTATION SPECIFIC NOTE
    // We have adopted the following approach in order to fix problem with Quint hashing:
    //  In the event when left branches are empty, 
    //  inner.prefix will be: InternalNodeHashPrefix.termConcat(Hash256_ZERO)
    //  Hash256_ZERO being termHash(raw([0]))
    //  which will result in: Map([0] -> Raw([0]), [1, 0] -> Raw([0]), [1] -> Hash).
    //  In Rust implementation this equals to:
    //  prefix: [ INTERNAL_NODE_HASH_PREFIX, sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).as_ref() ].concat()
    //        Note: here we are concatenating [0] and 
    //              [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    //  
    //  After concatanation, prefix will have lenght 33 and look like this:
    //  [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    //  It is obvious that this is simply an array of zeros with length 33.
    //  In Rust implementation of ics23, in leftBranchesAreEmpty branch of execution, this array is sliced like:
    //  op.prefix[from..from + child_size], `from` being `1` and `child_size` being 33. 
    //  Result of this slice should be equal to `spec.empty_child` which is Hash256::ZERO, 
    //  or simply put, the first zero which represents `INTERNAL_NODE_HASH_PREFIX`.
    //  However, this is not tha t simple to do in Quint.

    //  In order to do so, we have changed `termSlice(...)` function. When prefix has value
    //  Map([0] -> Raw([0]), [1, 0] -> Raw([0]), [1] -> Hash), 
    //  which is Quint equivalent of [ INTERNAL_NODE_HASH_PREFIX, sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).as_ref() ].concat(),
    //  we will mimic the behaviour of Rust code, 
    //  and simply just return `EmptyChild`, or: Map([0, 0] -> Raw([0]), [0] -> Hash), 
    //  which is the same as trimming of the first zero, which represents `INTERNAL_NODE_HASH_PREFIX`.
    
    val fromIndex = termLen(inner.prefix) - ChildSize
    termSlice(inner.prefix, fromIndex, fromIndex + ChildSize)== EmptyChild
  }

  /// IsLeftMost returns true if this is the left-most path in the tree,
  /// excluding placeholder (empty child) nodes
  def isLeftMost(path: List[INNER_T]): bool = {
    // Calls getPadding(0) => idx = 0, prefix = 0.
    // We specialize the constants to IavlSpec.
    // !!! UPDATE !!!
    // For Left Curve Grug we have changed: MinPrefixLen, MaxPrefixLen and ChildSize in order to
    // accustom their JellyFish Merkle Tree, 
    // therefore previous comment about specializing constants for IavlSpec is outdated
    path.indices().forall(i =>
      val pathStep = path[i]
      or {
        // the path goes left
        hasPadding(pathStep, MinPrefixLen, MaxPrefixLen, ChildSize),
        // the path goes right, but the left child is empty (a gap)
        leftBranchesAreEmpty(pathStep)
      }
    )
  }

  /// rightBranchesAreEmpty returns true if the padding bytes correspond
  /// to all empty siblings on the right side of a branch,
  /// i.e. it's a valid placeholder on a rightmost path
  def rightBranchesAreEmpty(inner: INNER_T): bool = and {
    // the case of rightBranches == 0 returns false
    val order = orderFromPadding(inner)
    order._2 and order._1 != 1,
    // the remaining case is rightBranches == 1, see orderFromPadding
    termLen(inner.suffix) == ChildSize,
    // getPosition(0) returns 0, hence, from == 0
    inner.suffix == EmptyChild
  }

  /// IsRightMost returns true if this is the left-most path in the tree,
  /// excluding placeholder (empty child) nodes
  def isRightMost(path: List[INNER_T]): bool = {
    // Specialize to IavlSpec
    // Calls getPadding(1) => minPrefix, maxPrefix,
    //   suffix = ChildSize + MinPrefixLen, ChildSize + MaxPrefixLen, 0
    // !!! UPDATE !!!
    // The same applies for Left Curve Grug JellyFish Merkle Tree
    path.indices().forall(i =>
      val pathStep = path[i]
      or {
        // the path goes right
        hasPadding(pathStep, ChildSize + MinPrefixLen, ChildSize + MaxPrefixLen, 0),
        // the path goes left, but the right child is empty (a gap)
        rightBranchesAreEmpty(pathStep)
      }
    )
  }

  /// isLeftStep assumes left and right have common parents
  /// checks if left is exactly one slot to the left of right
  def isLeftStep(left: INNER_T, right: INNER_T): bool = {
    // 'left' turns left, and 'right' turns right
    val lorder = orderFromPadding(left)
    val rorder = orderFromPadding(right)
    and {
      lorder._2,
      rorder._2,
      rorder._1 == lorder._1 + 1
    }
  }

  /// IsLeftNeighbor returns true if `right` is the next possible path
  /// right of `left`
  ///
  /// Find the common suffix from the Left.Path and Right.Path and remove it.
  /// We have LPath and RPath now, which must be neighbors.
  /// Validate that LPath[len-1] is the left neighbor of RPath[len-1].
  /// For step in LPath[0..len-1], validate step is right-most node.
  /// For step in RPath[0..len-1], validate step is left-most node.
  def isLeftNeighbor(lpath: List[INNER_T], rpath: List[INNER_T]): bool = {
    // count common tail (from end, near root)
    // cut the left and right paths
    lpath.indices().exists(li =>
      rpath.indices().exists(ri => and {
        // they are equidistant from the root
        length(lpath) - li == length(rpath) - ri,
        // The distance to the root (the indices are 0-based).
        // dist == 0 holds for the root.
        val dist = length(lpath) - 1 - li
        // the prefixes and suffixes match just above the cut points
        1.to(dist).forall(k =>
          val lnode = lpath[li + k]
          val rnode = rpath[ri + k]
          and {
            lnode.prefix == rnode.prefix,
            lnode.suffix == rnode.suffix
          }
        ),
        // Now topleft and topright are the first divergent nodes
        // make sure they are left and right of each other.
        // Actually, lpath[li] and rpath[ri] are an abstraction
        // of the same tree node:
        //  the left one stores the hash of the right one, whereas
        //  the right one stores the hash of the left one.
        isLeftStep(lpath[li], rpath[ri]),
        // left and right are remaining children below the split,
        // ensure left child is the rightmost path, and visa versa
        isRightMost(lpath.slice(0, li)),
        isLeftMost(rpath.slice(0, ri)),
      })
    )
  }

  /// VerifyNonMembership returns true iff
  /// proof is (contains) a NonExistenceProof,
  /// both left and right sub-proofs are valid existence proofs (see above) or nil,
  /// left and right proofs are neighbors (or left/right most if one is nil),
  /// provided key is between the keys of the two proofs
  def verifyNonMembership(root: CommitmentRoot_t,
      np: NonExistsProof_t, key: Key_t): bool = and {
    // getNonExistProofForKey
    np.left == None or lessThan(np.left.unwrap().key, key),
    np.right == None or lessThan(key, np.right.unwrap().key),
    // implicit assumption, missing in the code:
    // https://github.com/informalsystems/ics23-audit/issues/14
    np.key == key,
    // Verify
    np.left != None or np.right != None,
    np.left == None or and {
      verify(np.left.unwrap(), root, np.left.unwrap().key, np.left.unwrap().value), 
      lessThan(np.left.unwrap().key, key),
    },
    np.right == None or and {
      verify(np.right.unwrap(), root, np.right.unwrap().key, np.right.unwrap().value),
      lessThan(key, np.right.unwrap().key),
    },
    if (np.left == None) {
      isLeftMost(np.right.unwrap().path)
    } else if (np.right == None) {
      isRightMost(np.left.unwrap().path)
    } else {
      isLeftNeighbor(np.left.unwrap().path, np.right.unwrap().path)
    }
  }
}
