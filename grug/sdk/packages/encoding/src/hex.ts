import type { Hex } from "@leftcurve/types";

/**
 * Encode a byte array to a string using the Hex scheme, lowercase
 * @param bytes - The byte array to encode
 * @param prefixed - Whether to prefix the hex string with "0x"
 * @returns The hex string
 */
export function encodeHex(bytes: Uint8Array, prefixed = false): Hex {
  let hexStr = "";
  for (let i = 0; i < bytes.length; i++) {
    hexStr += bytes[i].toString(16).padStart(2, "0");
  }
  return prefixed ? `0x${hexStr}` : hexStr;
}

/**
 * Decode a string to byte array using the Hex scheme.
 * @param hex - The hex string to decode, with or without the "0x" prefix
 * @returns The byte array
 */
export function decodeHex(hex: Hex): Uint8Array {
  const hexStr = hex.startsWith("0x") ? hex.substring(2) : hex;
  if (hexStr.length % 2 !== 0) {
    throw new Error("hex string has an odd length");
  }
  const bytes = new Uint8Array(hexStr.length / 2);
  for (let i = 0, j = 0; i < hexStr.length; i += 2, j++) {
    const hexByteString = hexStr.substring(i, i + 2);
    if (!hexByteString.match(/[0-9a-f]{2}/i)) {
      throw new Error("invalid hex byte");
    }
    bytes[j] = Number.parseInt(hexByteString, 16);
  }
  return bytes;
}

/**
 * Check if a value is a hex string.
 * @param value - The value to check
 * @returns Whether the value is a hex string
 */
export function isHex(value: unknown): value is Hex {
  if (!value) return false;
  if (typeof value !== "string") return false;
  return value.startsWith("0x") || /^[0-9a-fA-F]*$/.test(value);
}

/**
 * Convert a hex string to a BigInt.
 * @param hex - The hex string to convert
 * @returns The BigInt
 */
export function hexToBigInt(hex: Hex): bigint {
  const hexStr = hex.startsWith("0x") ? hex : `0x${hex}`;
  return BigInt(hexStr);
}
