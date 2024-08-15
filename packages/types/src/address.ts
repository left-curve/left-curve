import { Sha256, ripemd160 } from "@leftcurve/crypto";
import {
  decodeHex,
  encodeHex,
  numberToUint8Array,
  stringToUint8ArrayWithLength,
} from "@leftcurve/encoding";

/**
 * Given parameters used while instantiating a new contract, compute what the
 * contract address would be.
 *
 * Mirrors that Rust function: `grug::Addr::compute`.
 *
 * @param deployer Address of the deployer, that it, the account that sends the
 * `Message::Instantiate`.
 * @param codeHash SHA-256 hash of the Wasm byte code.
 * @param salt An arbitrary byte array chosen by the deployer.
 * @returns The address that is given to the contract being instantiated.
 */
export function createAddress(deployer: string, codeHash: Uint8Array, salt: Uint8Array): string {
  const hasher = new Sha256();
  hasher.update(decodeHex(deployer.substring(2))); // strip the 0x prefix
  hasher.update(codeHash);
  hasher.update(salt);

  const bytes = hasher.digest();
  return "0x" + encodeHex(ripemd160(bytes));
}

/**
 * Given a username, keyId, and account type, create a salt to be used in the
 * creation of a new account.
 *
 * @param username The username of the account.
 * @param keyId The key ID of the account.
 * @param accountType The account type of the account.
 * @returns The salt to be used in the creation of a new account.
 */
export function createSalt(username: string, keyId: string, accountType: number) {
  const bytesUsername = stringToUint8ArrayWithLength(username);
  const bytesKeyId = decodeHex(keyId);
  const numberToBytes = numberToUint8Array(accountType, 1);

  const salt = new Uint8Array(bytesUsername.length + bytesKeyId.length + numberToBytes.length);

  salt.set(bytesUsername, 0);
  salt.set(bytesKeyId, bytesUsername.length);
  salt.set(numberToBytes, bytesUsername.length + bytesKeyId.length);

  return salt;
}
