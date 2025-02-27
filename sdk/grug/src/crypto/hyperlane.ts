import { concatBytes } from "@noble/hashes/utils";
import { encodeEndian32, encodeUtf8 } from "../encoding/index.js";
import { keccak256 } from "./sha.js";

export function multisigHash(
  domainHash: Uint8Array,
  merkleRoot: Uint8Array,
  merkleIndex: number,
  messageId: Uint8Array,
): Uint8Array {
  return keccak256(concatBytes(domainHash, merkleRoot, encodeEndian32(merkleIndex), messageId));
}

export function domainHash(domain: number, address: Uint8Array, key: string) {
  return keccak256(concatBytes(encodeEndian32(domain), address, encodeUtf8(key)));
}
