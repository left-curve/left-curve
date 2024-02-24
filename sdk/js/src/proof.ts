import { Sha256 } from "@cosmjs/crypto";
import { Hash, type MembershipProof, type NonMembershipProof, type Proof } from ".";

export function verifyProof(
  rootHash: Hash,
  keyHash: Hash,
  valueHash: Hash | null,
  proof: Proof,
) {
  if (valueHash !== null) {
    if ("membership" in proof) {
      return verifyMembershipProof(rootHash, keyHash, valueHash, proof.membership);
    } else {
      throw new Error("expecting membership proof, got non-membership proof");
    }
  } else {
    if ("nonMembership" in proof) {
      return verifyNonMembershipProof(rootHash, keyHash, proof.nonMembership);
    } else {
      throw new Error("expecting non-membership proof, got membership proof");
    }
  }
}

export function verifyMembershipProof(
  rootHash: Hash,
  keyHash: Hash,
  valueHash: Hash,
  proof: MembershipProof,
) {
  const hash = hashLeafNode(keyHash, valueHash);
  return computeAndCompareRootHash(rootHash, keyHash, proof.siblingHashes, hash);
}

export function verifyNonMembershipProof(
  rootHash: Hash,
  keyHash: Hash,
  proof: NonMembershipProof,
) {
  let hash: Hash;
  if ("internal" in proof.node) {
    const { leftHash, rightHash } = proof.node.internal;
    const bit = getBitAtIndex(keyHash, proof.siblingHashes.length);
    if ((bit === 0 && !!leftHash) || (bit === 1 && !!rightHash)) {
      throw new Error("expecting child to not exist but it exists");
    }
    hash = hashInternalNode(leftHash, rightHash);
  } else {
    const { keyHash: existKeyHash, valueHash } = proof.node.leaf;
    for (let i = proof.siblingHashes.length - 1; i >= 0; i--) {
      if (getBitAtIndex(keyHash, i) !== getBitAtIndex(existKeyHash, i)) {
        throw new Error("expecting bitarrays to share a common prefix but they do not");
      }
    }
    hash = hashLeafNode(existKeyHash, valueHash);
  }
  return computeAndCompareRootHash(rootHash, keyHash, proof.siblingHashes, hash);
}

function computeAndCompareRootHash(
  rootHash: Hash,
  keyHash: Hash,
  siblingHashes: (Hash | null)[],
  hash: Hash,
) {
  for (let i = siblingHashes.length - 1; i >= 0; i--) {
    if (getBitAtIndex(keyHash, i) == 0) {
      hash = hashInternalNode(hash, siblingHashes[i]);
    } else {
      hash = hashInternalNode(siblingHashes[i], hash);
    }
  }

  for (let i = 0; i < 32; i++) {
    if (rootHash.bytes[i] !== hash.bytes[i]) {
      throw new Error(`root hash mismatch! computed: ${hash.toHex()}, actual: ${rootHash.toHex()}`);
    }
  }
}

function hashInternalNode(leftHash: Hash | null, rightHash: Hash | null): Hash {
  const hasher = new Sha256();
  hasher.update(new Uint8Array([0])); // internal node prefix
  if (leftHash !== null) {
    hasher.update(leftHash.bytes);
  } else {
    hasher.update(new Uint8Array(32)); // this creates an all-zero byte array
  }
  if (rightHash !== null) {
    hasher.update(rightHash.bytes);
  } else {
    hasher.update(new Uint8Array(32));
  }
  return new Hash(hasher.digest());
}

function hashLeafNode(keyHash: Hash, valueHash: Hash): Hash {
  const hasher = new Sha256();
  hasher.update(new Uint8Array([1])); // leaf node prefix
  hasher.update(keyHash.bytes);
  hasher.update(valueHash.bytes);
  return new Hash(hasher.digest());
}

function getBitAtIndex(hash: Hash, index: number): number {
  const quotient = Math.floor(index / 8);
  const remainder = index % 8;
  const byte = hash.bytes[quotient];
  return byte >> (7 - remainder) & 1;
}
