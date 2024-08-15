import { Sha256, ripemd160 } from "@leftcurve/crypto";
import {
  decodeHex,
  encodeEndian32,
  encodeHex,
  encodeUtf8,
  numberToUint8Array,
  serialize,
  stringToUint8ArrayWithLength,
} from "@leftcurve/encoding";
import type { AbstractSigner, Account, Address, Hex, Message, Metadata } from "@leftcurve/types";

export function toAccount({
  username,
  signer,
}: { username: string; signer: AbstractSigner }): Account {
  async function computeAddress(
    username: string,
    factoryAddr: string,
    accountTypeCodeHash: string,
  ): Promise<Address> {
    const keyId = await signer.getKeyId();
    const salt = createSalt(username, keyId, 0);
    return createAddress(factoryAddr, decodeHex(accountTypeCodeHash), salt);
  }

  async function signTx(msgs: Message[], chainId: string, sequence: number) {
    const credential = await signer.signTx(msgs, chainId, sequence);
    const data: Metadata = { username, keyId: await signer.getKeyId(), sequence };

    return { credential, data };
  }

  return {
    username,
    computeAddress,
    getKeyId: () => signer.getKeyId(),
    signTx,
  };
}

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
export function createAddress(deployer: string, codeHash: Uint8Array, salt: Uint8Array): Address {
  const hasher = new Sha256();
  hasher.update(decodeHex(deployer.substring(2))); // strip the 0x prefix
  hasher.update(codeHash);
  hasher.update(salt);

  const bytes = hasher.digest();
  return `0x${encodeHex(ripemd160(bytes))}`;
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
export function createSalt(username: string, keyId: Hex, accountType: number) {
  const bytesUsername = stringToUint8ArrayWithLength(username);
  const bytesKeyId = typeof keyId === "string" ? decodeHex(keyId) : keyId;
  const numberToBytes = numberToUint8Array(accountType, 1);

  const salt = new Uint8Array(bytesUsername.length + bytesKeyId.length + numberToBytes.length);

  salt.set(bytesUsername, 0);
  salt.set(bytesKeyId, bytesUsername.length);
  salt.set(numberToBytes, bytesUsername.length + bytesKeyId.length);

  return salt;
}

/**
 * Generate sign byte that the grug-account contract expects.
 *
 * Mirrors the Rust function: `grug_account::sign_bytes`.
 */
export function createSignBytes(
  msgs: Message[],
  sender: string,
  chainId: string,
  sequence: number,
): Uint8Array {
  const hasher = new Sha256();
  hasher.update(serialize(msgs));
  hasher.update(decodeHex(sender.substring(2))); // strip the 0x prefix
  hasher.update(encodeUtf8(chainId));
  hasher.update(encodeEndian32(sequence));
  return hasher.digest();
}
