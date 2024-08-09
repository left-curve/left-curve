/**
 * Encode a byte array to a string using the Hex scheme, lowercase, no 0x prefix.
 */
export function encodeHex(bytes: Uint8Array): string {
  let hexStr = "";
  for (let i = 0; i < bytes.length; i++) {
    hexStr += bytes[i].toString(16).padStart(2, "0");
  }
  return hexStr;
}

/**
 * Decode a string to byte array using the Hex scheme.
 */
export function decodeHex(hexStr: string): Uint8Array {
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
