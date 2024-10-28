import type { Base64, Hex } from "./encoding.js";

export const KeyAlgo = {
  Secp256r1: "secp256r1",
  Secp256k1: "secp256k1",
  Ed25519: "ed25519",
} as const;

export type KeyAlgoType = (typeof KeyAlgo)[keyof typeof KeyAlgo];

export type KeyHash = Hex;

export const KeyTag = {
  secp256r1: 0,
  secp256k1: 1,
  ed25519: 2,
} as const;

/** A public key that can be associated with an Account */
export type Key =
  /** An Secp256k1 public key in compressed form. */
  | { secp256k1: Base64 }
  /** An Ed25519 public key. */
  | { ed25519: Base64 }
  /** An Secp256r1 public key in compressed form. */
  | { secp256r1: Base64 };
