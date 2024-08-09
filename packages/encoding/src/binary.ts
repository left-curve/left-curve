import { type Payload, camelToSnake, recursiveTransform, snakeToCamel } from "@leftcurve/utils";
import { decodeUtf8, encodeUtf8 } from "./utf8";

/**
 * Serialize a message to binary.
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
export function deserialize<T = Payload>(bytes: Uint8Array): T {
  return recursiveTransform(JSON.parse(decodeUtf8(bytes)), snakeToCamel) as unknown as T;
}
