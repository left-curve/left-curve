import type { Hash } from ".";

export type Proof = { membership: MembershipProof } | { nonMembership: NonMembershipProof };

export type MembershipProof = {
  siblingHashes: (Hash | null)[];
};

export type NonMembershipProof = {
  node: Node;
  siblingHashes: (Hash | null)[];
};

export type Node = { internal: InternalNode } | { leaf: LeafNode };

export type InternalNode = {
  leftHash: Hash | null;
  rightHash: Hash | null;
};

export type LeafNode = {
  keyHash: Hash;
  valueHash: Hash;
};
