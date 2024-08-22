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

export function serialize(payload: any): Uint8Array {
  return encodeUtf8(JSON.stringify(recursiveTransform(payload, camelToSnake)));
}

/**
 * Deserialize a JSON string to a payload. The reverse operation of `serialize`.
 */
export function deserialize<T extends Json>(bytes: Uint8Array): T {
  return recursiveTransform(JSON.parse(decodeUtf8(bytes)), snakeToCamel) as unknown as T;
}
