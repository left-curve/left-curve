import { Sha256 } from "@left-curve/sdk/crypto";
import { decodeHex, encodeEndian32, encodeUtf8, serialize } from "@left-curve/sdk/encoding";
import type { Address, Message } from "@left-curve/types";

/**
 * Generate sign byte that the grug-account contract expects.
 *
 * Mirrors the Rust function: `grug_account::sign_bytes`.
 */
export function createSignBytes(
  msgs: Message[],
  sender: Address,
  chainId: string,
  sequence: number,
): Uint8Array {
  const hasher = new Sha256();
  return hasher
    .update(serialize(msgs))
    .update(decodeHex(sender.substring(2))) // strip the 0x prefix
    .update(encodeUtf8(chainId))
    .update(encodeEndian32(sequence))
    .digest();
}
