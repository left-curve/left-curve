/**
 * Given a number, assume it is a non-negative integer, encode it as 32-bit big
 * endian bytes.
 */
export function encodeBigEndian32(value: number): Uint8Array {
  const view = new DataView(new ArrayBuffer(4));
  view.setUint32(0, value, false);
  return new Uint8Array(view.buffer);
}

/**
 * Given a byte array, attempt to deserialize it into a number as 32-bit big
 * endian encoding. Error if the byte array isn't exactly 4 bytes in length.
 */
export function decodeBigEndian32(bytes: Uint8Array): number {
  if (bytes.byteLength !== 4) {
    throw new Error(`expecting exactly 4 bytes, got ${bytes.byteLength}`);
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return view.getUint32(0, false);
}
