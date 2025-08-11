import type { Address, Base64, DateTime, Hex } from "@left-curve/sdk/types";

export type KeyHash = Hex;

export const KeyTag = {
  secp256r1: 0,
  secp256k1: 1,
  ethereum: 2,
} as const;

/** A public key that can be associated with an Account */
export type Key =
  /** An Secp256k1 public key in compressed form. */
  | { secp256k1: Base64 }
  /** An Ethereum address. */
  | { ethereum: Address }
  /** An Secp256r1 public key in compressed form. */
  | { secp256r1: Base64 };

export type PublicKey = {
  /** The key hash of the public key */
  keyHash: KeyHash;
  /** The public key */
  publicKey: Hex;
  /** The type of the public key */
  keyType: Uppercase<keyof typeof KeyTag>;
  /** The block height at which the key was created */
  createdBlockHeight: number;
  /** The timestamp at which the key was created */
  createdAt: DateTime;
};
