import { decodeBase64, decodeHex, encodeEndian32 } from "@left-curve/sdk/encoding";
import type { Key, KeyHash } from "../types/key.js";
import { KeyTag } from "../types/key.js";

type CreateAccountSaltParameters = {
  key: Key;
  keyHash: KeyHash;
  seed: number;
};

/**
 * Given a username, and a key, create a salt to be used in the
 * creation of a new account.
 *
 * @param parameters The parameters to create the user account salt.
 * @param parameters.key The key to create the account salt for.
 * @param parameters.keyHash The key hash to create the account salt for.
 * @param parameters.seed The seed to create the account salt for.
 * @returns The salt to be used in the creation of a new user account.
 */

export function createAccountSalt(parameters: CreateAccountSaltParameters): Uint8Array {
  const { key, keyHash, seed } = parameters;
  const [keyTag, publicKey] = Object.entries(key)[0];
  const publicKeyBytes = decodeBase64(publicKey);
  const bytes: number[] = [];
  bytes.push(...encodeEndian32(seed));
  bytes.push(...decodeHex(keyHash));
  bytes.push(KeyTag[keyTag as keyof typeof KeyTag]);
  bytes.push(...publicKeyBytes);
  return new Uint8Array(bytes);
}
