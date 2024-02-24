import type { Hash } from ".";

export type ProofNode = {
  internal: {
    leftHash: Hash | null;
    rightHash: Hash | null;
  };
} | {
  leaf: {
    keyHash: Hash;
    valueHash: Hash;
  };
};

export type Proof = {
  membership: MembershipProof;
} | {
  nonMembership: NonMembershipProof;
};

export type MembershipProof = {
  siblingHashes: (Hash | null)[];
};

export type NonMembershipProof = {
  node: ProofNode;
  siblingHashes: (Hash | null)[];
};
