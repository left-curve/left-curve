import type { Binary, Json, JsonValue } from "../types/index.js";
import { camelToSnake, recursiveTransform, snakeToCamel } from "../utils/index.js";
import { sortedJsonStringify } from "./json.js";
import { decodeUtf8, encodeUtf8 } from "./utf8.js";

/**
 * Serialize a message to binary.
 *
 * The payload is first converted to snake_case, encoded to a JSON string, then
 * to UTF8 bytes.
 */
export function serialize(payload: Json | JsonValue): Binary {
  return encodeUtf8(sortedJsonStringify(recursiveTransform(payload, camelToSnake)));
}

/**
 * Deserialize a JSON string to a payload. The reverse operation of `serialize`.
 */
export function deserialize<T>(bytes: Binary): T {
  return recursiveTransform(JSON.parse(decodeUtf8(bytes)), snakeToCamel) as unknown as T;
}
