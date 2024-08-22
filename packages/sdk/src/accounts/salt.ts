import { ripemd160 } from "@leftcurve/crypto";
import { decodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { type AccountIndex, type Key, KeyTag, type Username } from "@leftcurve/types";

/**
 * Given a username, and a key, create a salt to be used in the
 * creation of a new account.
 *
 * @param username The username of the account.
 * @param accountIndex The index of the account.
 * @param key The key to be associated with the account.
 * @returns The salt to be used in the creation of a new account.
 */
export function createAccountSalt(username: Username, accountIndex: AccountIndex, key: Key) {
  if (accountIndex > 1) return encodeUtf8(`${username}/account/${accountIndex}`);

  if (username.length > 255) throw new Error("Username is too long");
  const [keyTag, publicKey] = Object.entries(key)[0];
  const publicKeyBytes = decodeBase64(publicKey);
  const bytes: number[] = [];
  bytes.push(username.length);
  bytes.push(...encodeUtf8(username));
  bytes.push(...ripemd160(publicKeyBytes));
  bytes.push(KeyTag[keyTag as keyof typeof KeyTag]);
  bytes.push(...publicKeyBytes);
  return new Uint8Array(bytes);
}
