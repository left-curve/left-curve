/**
 * Encodes a 32-bit unsigned integer into a byte array in little-endian or big-endian order.
 * @param value - The number to encode.
 * @param littleEndian - Whether to encode in little-endian order. Defaults to big-endian.
 */
export function encodeEndian32(value: number, littleEndian?: boolean): Uint8Array {
  const view = new DataView(new ArrayBuffer(4));
  view.setUint32(0, value, littleEndian);
  return new Uint8Array(view.buffer);
}

/**
 * Decodes a 32-bit unsigned integer from a byte array in little-endian or big-endian order.
 * @param bytes - The byte array to decode.
 * @param littleEndian - Whether the byte array is in little-endian order. Defaults to big-endian.
 */
export function decodeEndian32(bytes: Uint8Array, littleEndian?: boolean): number {
  if (bytes.byteLength !== 4) {
    throw new Error(`expecting exactly 4 bytes, got ${bytes.byteLength}`);
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return view.getUint32(0, littleEndian);
}
