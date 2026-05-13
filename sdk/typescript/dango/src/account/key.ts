import { sha256 } from "@left-curve/crypto";
import { encodeHex, encodeUtf8 } from "@left-curve/encoding";
import type { KeyHash } from "@left-curve/types";

/**
 * Create a key hash from a public key or credential id.
 * @param s The source to create the key hash from
 * @returns The key hash
 */
export function createKeyHash(s: string | Uint8Array): KeyHash {
  const buff = typeof s === "string" ? encodeUtf8(s) : s;

  return encodeHex(sha256(buff)).toUpperCase();
}
