export type Proof = { membership: MembershipProof } | { nonMembership: NonMembershipProof };

export type MembershipProof = {
  siblingHashes: (string | null)[];
};

export type NonMembershipProof = {
  node: Node;
  siblingHashes: (string | null)[];
};

export type Node = { internal: InternalNode } | { leaf: LeafNode };

export type InternalNode = {
  leftHash: string | null;
  rightHash: string | null;
};

export type LeafNode = {
  keyHash: string;
  valueHash: string;
};
