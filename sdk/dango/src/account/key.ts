import { secp256k1CompressPubKey, sha256 } from "@left-curve/sdk/crypto";
import { encodeHex, encodeUtf8 } from "@left-curve/sdk/encoding";
import type { OneRequired, Prettify } from "@left-curve/sdk/types";
import type { KeyHash } from "../types/key.js";

type CreateKeyHashParameters = Prettify<
  OneRequired<
    {
      pubKey: Uint8Array;
      credentialId: string;
    },
    "pubKey",
    "credentialId"
  >
>;

/**
 * Create a key hash from a public key or credential id.
 * @param parameters The parameters to create the key hash
 * @param parameters.pubKey The public key to create the key hash from
 * @param parameters.credentialId The credential ID to create the key hash from
 * @returns The key hash
 */
export function createKeyHash(parameters: CreateKeyHashParameters): KeyHash {
  const { pubKey, credentialId } = parameters;

  if (credentialId) {
    return encodeHex(sha256(encodeUtf8(credentialId))).toUpperCase();
  }

  if (!pubKey) throw new Error("no public key or credential ID provided");

  const compressedKey = secp256k1CompressPubKey(pubKey, true);

  return encodeHex(sha256(compressedKey)).toUpperCase();
}
