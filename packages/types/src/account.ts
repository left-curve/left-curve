import { Sha256 } from "@leftcurve/crypto";
import { decodeHex, encodeEndian32, encodeUtf8, serialize } from "@leftcurve/encoding";
import type { Base64 } from "./common";
import type { Credential, Message, Metadata } from "./tx";

export type PublicKey = { secp256k1: Base64 } | { secp256r1: Base64 };

export type Account = {
  address: string;
  signTx: (
    msgs: Message[],
    sender: string,
    chainId: string,
    accountState: { sequence: number },
  ) => Promise<{ credential: Credential; data: Metadata }>;
};

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
