import { Sha256 } from "@left-curve/crypto";
import { decodeHex, encodeHex } from "@left-curve/encoding";
import type { MembershipProof, NonMembershipProof, Proof } from "@left-curve/types";

export function verifyProof(
  rootHash: Uint8Array,
  keyHash: Uint8Array,
  valueHash: Uint8Array | null,
  proof: Proof,
) {
  // value exists, the proof must be a membership proof
  if (valueHash !== null) {
    if ("membership" in proof) {
      return verifyMembershipProof(rootHash, keyHash, valueHash, proof.membership);
    }
    throw new Error("expecting membership proof, got non-membership proof");
  }
  // value doesn't exist, the proof must be a non-membership proof
  if ("nonMembership" in proof) {
    return verifyNonMembershipProof(rootHash, keyHash, proof.nonMembership);
  }
  throw new Error("expecting non-membership proof, got membership proof");
}

export function verifyMembershipProof(
  rootHash: Uint8Array,
  keyHash: Uint8Array,
  valueHash: Uint8Array,
  proof: MembershipProof,
) {
  const hash = hashLeafNode(keyHash, valueHash);
  return computeAndCompareRootHash(rootHash, keyHash, proof.siblingHashes, hash);
}

export function verifyNonMembershipProof(
  rootHash: Uint8Array,
  keyHash: Uint8Array,
  proof: NonMembershipProof,
) {
  let hash: Uint8Array;
  if ("internal" in proof.node) {
    const { leftHash, rightHash } = proof.node.internal;
    const bit = getBitAtIndex(keyHash, proof.siblingHashes.length);
    if ((bit === 0 && !!leftHash) || (bit === 1 && !!rightHash)) {
      throw new Error("expecting child to not exist but it exists");
    }
    hash = hashInternalNode(decodeNullableHex(leftHash), decodeNullableHex(rightHash));
  } else {
    const existingKeyHash = decodeHex(proof.node.leaf.keyHash);
    for (let i = proof.siblingHashes.length - 1; i >= 0; i--) {
      if (getBitAtIndex(keyHash, i) !== getBitAtIndex(existingKeyHash, i)) {
        throw new Error("expecting bitarrays to share a common prefix but they do not");
      }
    }
    hash = hashLeafNode(existingKeyHash, decodeHex(proof.node.leaf.valueHash));
  }
  return computeAndCompareRootHash(rootHash, keyHash, proof.siblingHashes, hash);
}

function computeAndCompareRootHash(
  rootHash: Uint8Array,
  keyHash: Uint8Array,
  siblingHashes: (string | null)[],
  hash: Uint8Array,
) {
  for (let i = 0; i < siblingHashes.length; i++) {
    if (getBitAtIndex(keyHash, siblingHashes.length - i - 1) === 0) {
      hash = hashInternalNode(hash, decodeNullableHex(siblingHashes[i]));
    } else {
      hash = hashInternalNode(decodeNullableHex(siblingHashes[i]), hash);
    }
  }

  for (let i = 0; i < 32; i++) {
    if (rootHash[i] !== hash[i]) {
      throw new Error(
        `root hash mismatch! computed: ${encodeHex(hash)}, actual: ${encodeHex(rootHash)}`,
      );
    }
  }
}

function hashInternalNode(leftHash: Uint8Array | null, rightHash: Uint8Array | null): Uint8Array {
  const hasher = new Sha256();
  hasher.update(new Uint8Array([0])); // internal node prefix
  if (leftHash !== null) {
    hasher.update(leftHash);
  } else {
    hasher.update(new Uint8Array(32)); // this creates an all-zero byte array
  }
  if (rightHash !== null) {
    hasher.update(rightHash);
  } else {
    hasher.update(new Uint8Array(32));
  }
  return hasher.digest();
}

function hashLeafNode(keyHash: Uint8Array, valueHash: Uint8Array): Uint8Array {
  const hasher = new Sha256();
  hasher.update(new Uint8Array([1])); // leaf node prefix
  hasher.update(keyHash);
  hasher.update(valueHash);
  return hasher.digest();
}

function getBitAtIndex(hash: Uint8Array, index: number): number {
  const quotient = Math.floor(index / 8);
  const remainder = index % 8;
  const byte = hash[quotient];
  return (byte >> (7 - remainder)) & 1;
}

function decodeNullableHex(hexStr: string | null): Uint8Array | null {
  return hexStr !== null ? decodeHex(hexStr) : null;
}
