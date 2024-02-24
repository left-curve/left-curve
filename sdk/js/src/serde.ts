import type { Addr, Hash, Uint } from ".";

/**
 * Represents either an JSON object, an array, a string, or a number.
 * Note that we utilize a recursive type definition here.
 */
export type Payload = { [key: string]: Payload } | Payload[] | string | number | Addr | Hash | Uint;

/**
 * Serialize a payload to binary.
 *
 * The payload is first converted to snake_case, encoded to a JSON string, then
 * to UTF8 bytes.
 */
export function serialize(payload: Payload): Uint8Array {
  return encodeUtf8(JSON.stringify(recursiveTransform(payload, camelToSnake)));
}

/**
 * Deserialize a JSON string to a payload. The reverse operation of `serialize`.
 */
export function deserialize(bytes: Uint8Array): Payload {
  return recursiveTransform(JSON.parse(decodeUtf8(bytes)), snakeToCamel);
}

/**
 * Given a payload, recursively transform the case of the keys.
 *
 * To transform camelCase to snake_case, do:
 *
 * ```javascript
 * let snakeCasePayload = recursiveTransform(payload, camelToSnake);
 * ```
 *
 * To transform snake_case to camelCase, do:
 *
 * ```javascript
 * let camelCasePayload = recursiveTransform(payload, snakeToCamel);
 * ```
 */
export function recursiveTransform(
  payload: Payload,
  transformFn: (str: string) => string,
): Payload {
  // for strings, numbers, and nulls, there's no key to be transformed
  if (typeof payload !== "object" || payload === null) {
    return payload;
  }

  // for arrays, we recursively transform each element
  if (Array.isArray(payload)) {
    return payload.map((element) => recursiveTransform(element, transformFn));
  }

  // for objects, we go through each key, transforming it to snake_case
  const newObj = {} as { [key: string]: Payload };
  for (const [key, value] of Object.entries(payload)) {
    newObj[transformFn(key)] = recursiveTransform(value, transformFn);
  }
  return newObj;
}

/**
 * Convert a string from camelCase to snake_case.
 */
export function camelToSnake(str: string): string {
  return str.replace(/([A-Z])/g, "_$1").toLowerCase();
}

/**
 * Convert a string from snake_case to camelCase.
 */
export function snakeToCamel(str: string): string {
  return str.replace(/(_[a-z])/g, (group) => group.toUpperCase().replace("_", ""));
}

/**
 * Encode a string to to UTF-8 bytes.
 */
export function encodeUtf8(str: string): Uint8Array {
  const encoder = new TextEncoder();
  return encoder.encode(str);
}

/**
 * Decode UTF-8 bytes into a string.
 */
export function decodeUtf8(bytes: Uint8Array): string {
  const decoder = new TextDecoder();
  return decoder.decode(bytes);
}

/**
 * Given a number, assume it is a non-negative integer, encode it as 32-bit big
 * endian bytes.
 */
export function encodeBigEndian32(value: number): Uint8Array {
  const buffer = new ArrayBuffer(4);
  const view = new DataView(buffer);
  view.setUint32(0, value, false);
  return new Uint8Array(buffer);
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
    bytes[j] = parseInt(hexStr.substring(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Encode a byte array to a string using the Base64 scheme.
 *
 * JavaScript provides the built-in `btoa` function, but it only works with
 * strings, so we first need to convert the byte array to a Unicode string.
 */
export function encodeBase64(bytes: Uint8Array): string {
  let unicodeStr = "";
  for (let i = 0; i < bytes.length; i++) {
    unicodeStr += String.fromCharCode(bytes[i]);
  }
  return btoa(unicodeStr);
}

/**
 * Decode a string to byte array using the Base64 scheme.
 *
 * JavaScript provides the build-in `atob` function, but it only works with
 * strings, so we first need to convert the Base64 string to a Unicode string.
 */
export function decodeBase64(base64Str: string): Uint8Array {
  const unicodeStr = atob(base64Str);
  const bytes = new Uint8Array(unicodeStr.length);
  for (let i = 0; i < unicodeStr.length; i++) {
    bytes[i] = unicodeStr.charCodeAt(i);
  }
  return bytes;
}
