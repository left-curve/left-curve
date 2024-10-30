import { decodeBase64, decodeHex, encodeUtf8 } from "@leftcurve/encoding";
import {
  type AccountIndex,
  type Key,
  type KeyHash,
  KeyTag,
  type OneRequired,
  type Username,
} from "@leftcurve/types";

type CreateAccountSaltParameters = {
  key: Key;
  username: Username;
} & OneRequired<{ accountIndex: AccountIndex; keyHash: KeyHash }, "accountIndex", "keyHash">;

/**
 * Given a username, and a key, create a salt to be used in the
 * creation of a new account.
 *
 * @param parameters The parameters to create the account salt.
 * @param parameters.username The username to create the account salt for.
 * @param parameters.accountIndex The account index to create the account salt for.
 * @param parameters.key The key to create the account salt for.
 * @param parameters.keyHash The key hash to create the account salt for.
 * @returns The salt to be used in the creation of a new account.
 */
export function createAccountSalt(parameters: CreateAccountSaltParameters): Uint8Array {
  const { username, accountIndex, key, keyHash } = parameters;
  if (username.length > 255) throw new Error("Username is too long");

  if (accountIndex) {
    const bytes: number[] = [];
    bytes.push(username.length);
    bytes.push(...encodeUtf8(username));
    bytes.push(accountIndex);
    return new Uint8Array(bytes);
  }

  if (keyHash) {
    const [keyTag, publicKey] = Object.entries(key)[0];
    const publicKeyBytes = decodeBase64(publicKey);
    const bytes: number[] = [];
    bytes.push(username.length);
    bytes.push(...encodeUtf8(username));
    bytes.push(...decodeHex(keyHash));
    bytes.push(KeyTag[keyTag as keyof typeof KeyTag]);
    bytes.push(...publicKeyBytes);
    return new Uint8Array(bytes);
  }

  throw new Error("No account index or key hash provided");
}
