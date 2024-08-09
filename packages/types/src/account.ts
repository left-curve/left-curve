import { Sha256 } from "@leftcurve/crypto";
import { decodeHex, encodeBigEndian32, encodeUtf8, serialize } from "@leftcurve/encoding";
import type { Message } from "./tx";

export type PublicKey = { secp256k1: string } | { secp256r1: string };

export type AccountFactoryExecuteMsg = {
  registerAccount?: MsgRegisterAccount;
};

export type MsgRegisterAccount = {
  codeHash: string;
  publicKey: PublicKey;
};

export type AccountStateResponse = {
  publicKey: PublicKey;
  sequence: number;
};

/**
 * Derive the salt that is used by the standard account factory contract to
 * register accounts.
 *
 * Mirrors the Rust function: `grug_account_factory::make_salt`.
 *
 * @param publicKeyType A string identifying the type of public key. Can either
 * be `secp256k1` or `secp256r1`.
 * @param publicKeyBytes The public key as a byte array.
 * @param serial A serial number. The account to be created under this public
 * key gets serial 0, the second gets 1, so on.
 * @returns The salt that the account factory will use to instantiate the account.
 */
export function createSalt(
  publicKeyType: "secp256k1" | "secp256r1",
  publicKeyBytes: Uint8Array,
  serial: number,
): Uint8Array {
  const hasher = new Sha256();
  hasher.update(encodeUtf8(publicKeyType));
  hasher.update(publicKeyBytes);
  hasher.update(encodeBigEndian32(serial));
  return hasher.digest();
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
  hasher.update(encodeBigEndian32(sequence));
  return hasher.digest();
}
