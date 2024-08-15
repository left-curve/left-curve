import { camelToSnake, recursiveTransform, snakeToCamel } from "@leftcurve/utils";
import { decodeUtf8, encodeUtf8 } from "./utf8";

/**
 * Represents either an JSON object, an array, a string, a number, a null, an undefined or a boolean.
 * Note that we utilize a recursive type definition here.
 */
type Json = { [key: string]: Json } | Json[] | string | number | boolean | undefined | null;

/**
 * Serialize a message to binary.
 *
 * The payload is first converted to snake_case, encoded to a JSON string, then
 * to UTF8 bytes.
 */

// biome-ignore lint/suspicious/noExplicitAny: This is a generic function that can take any type.
export function serialize(payload: any): Uint8Array {
  return encodeUtf8(JSON.stringify(recursiveTransform(payload, camelToSnake)));
}

/**
 * Deserialize a JSON string to a payload. The reverse operation of `serialize`.
 */
export function deserialize<T extends Json>(bytes: Uint8Array): T {
  return recursiveTransform(JSON.parse(decodeUtf8(bytes)), snakeToCamel) as unknown as T;
}

/**
 *  Encodes a string to a Uint8Array with a length prefix.
 * @param str - The string to encode.
 * @returns The encoded Uint8Array.
 */

export function stringToUint8ArrayWithLength(str: string) {
  const bytesStr = encodeUtf8(str);
  const bytesLength = numberToUint8Array(bytesStr.length, 4);
  const result = new Uint8Array(bytesLength.length + bytesStr.length);
  result.set(bytesLength, 0);
  result.set(bytesStr, bytesLength.length);
  return result;
}

/**
 * Encodes a number to a Uint8Array with a fixed length.
 * @param num - The number to encode.
 * @param length - The number of bytes to encode the number into.
 * @returns The encoded Uint8Array.
 */
export function numberToUint8Array(num: number, length: number) {
  const array = new Uint8Array(length);
  for (let i = 0; i < length; i++) {
    array[i] = (num >> (8 * i)) & 0xff;
  }
  return array;
}
