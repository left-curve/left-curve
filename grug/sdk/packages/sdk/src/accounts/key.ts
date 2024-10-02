import { compressPubKey, ripemd160 } from "@leftcurve/crypto";
import { encodeHex, encodeUtf8 } from "@leftcurve/encoding";
import type { KeyHash, OneRequired } from "@leftcurve/types";

type CreateKeyHashParameters = OneRequired<
  {
    pubKey: Uint8Array;
    credentialId: string;
  },
  "pubKey",
  "credentialId"
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
  if (pubKey) {
    const compressedKey = compressPubKey(pubKey, true);
    return encodeHex(ripemd160(compressedKey)).toUpperCase();
  }

  if (credentialId) {
    return encodeHex(ripemd160(encodeUtf8(credentialId))).toUpperCase();
  }

  throw new Error("No public key or credential ID provided");
}
