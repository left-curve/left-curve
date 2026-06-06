import { secp256k1CompressPubKey } from "@left-curve/crypto";
import { decodeBase64, decodeHex, encodeBase64 } from "@left-curve/encoding";
import { createKeyHash } from "@left-curve/sdk";

import type { Hex, KeyHash } from "@left-curve/types";

export type ParsedSecp256k1PublicKey = {
  publicKey: Uint8Array;
  publicKeyBase64: string;
  keyHash: KeyHash;
};

const HEX_RE = /^[0-9a-fA-F]+$/;

function normalizeInput(input: string) {
  return input.trim().replace(/\s+/g, "");
}

function padBase64(input: string) {
  const remainder = input.length % 4;
  if (remainder === 0 || remainder === 1) return input;
  return input.padEnd(input.length + 4 - remainder, "=");
}

function normalizePublicKey(bytes: Uint8Array): ParsedSecp256k1PublicKey {
  const uncompressedPublicKey = secp256k1CompressPubKey(bytes, false);
  const publicKey = secp256k1CompressPubKey(uncompressedPublicKey, true);

  if (publicKey.length !== 33) {
    throw new Error("Invalid secp256k1 public key length");
  }

  return {
    publicKey,
    publicKeyBase64: encodeBase64(publicKey),
    keyHash: createKeyHash(publicKey),
  };
}

export function parseSecp256k1PublicKey(input: string): ParsedSecp256k1PublicKey | null {
  const normalized = normalizeInput(input);
  if (!normalized) return null;

  const hasHexPrefix = normalized.startsWith("0x") || normalized.startsWith("0X");
  const mayBeHex = hasHexPrefix || HEX_RE.test(normalized);

  if (mayBeHex) {
    try {
      const hex = hasHexPrefix ? `0x${normalized.slice(2)}` : normalized;
      return normalizePublicKey(decodeHex(hex as Hex));
    } catch {
      if (hasHexPrefix) return null;
    }
  }

  try {
    return normalizePublicKey(decodeBase64(padBase64(normalized)));
  } catch {
    return null;
  }
}
