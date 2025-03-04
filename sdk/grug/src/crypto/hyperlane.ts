import { encodeEndian32, encodeUtf8 } from "../encoding/index.js";
import { keccak256 } from "./sha.js";

export function multisigHash(
  domainHash: Uint8Array,
  merkleRoot: Uint8Array,
  merkleIndex: number,
  messageId: Uint8Array,
): Uint8Array {
  const bytes: number[] = [];
  bytes.push(...domainHash);
  bytes.push(...merkleRoot);
  bytes.push(...encodeEndian32(Number(merkleIndex)));
  bytes.push(...messageId);
  return keccak256(new Uint8Array(bytes));
}

export function domainHash(domain: number, address: Uint8Array, key: string) {
  let offset = 0;
  const buff = new Uint8Array(36 + key.length);
  buff.set(encodeEndian32(domain), offset);
  offset += 4;
  buff.set(address, offset);
  offset += 32;
  buff.set(encodeUtf8(key), offset);
  return keccak256(buff);
}
