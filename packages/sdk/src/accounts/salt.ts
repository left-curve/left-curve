import { decodeHex, numberToUint8Array, stringToUint8ArrayWithLength } from "@leftcurve/encoding";
import type { Hex } from "@leftcurve/types";

/**
 * Given a username, keyId, and account type, create a salt to be used in the
 * creation of a new account.
 *
 * @param username The username of the account.
 * @param keyId The key ID of the account.
 * @param accountType The account type of the account.
 * @returns The salt to be used in the creation of a new account.
 */
export function createAccountSalt(username: string, keyId: Hex, accountType: number) {
  const bytesUsername = stringToUint8ArrayWithLength(username);
  const bytesKeyId = typeof keyId === "string" ? decodeHex(keyId) : keyId;
  const numberToBytes = numberToUint8Array(accountType, 1);

  const salt = new Uint8Array(bytesUsername.length + bytesKeyId.length + numberToBytes.length);

  salt.set(bytesUsername, 0);
  salt.set(bytesKeyId, bytesUsername.length);
  salt.set(numberToBytes, bytesUsername.length + bytesKeyId.length);

  return salt;
}
