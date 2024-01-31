/**
 * Represents either an JSON object, a string, or a number.
 * Note that we utilize a recursive type definition here.
 */
export type Message = { [key: string]: Message } | string | number;

/**
 * Serialize a `Message` to JSON string.
 */
export function serialize(msg: Message): string {
  if (typeof msg === "string" || typeof msg === "number") {
    return btoa(JSON.stringify(msg));
  } else {
    let snakeCaseObj = {} as { [key: string]: Message | string | number };
    for (const key of Object.keys(msg)) {
      const snakeKey = camelToSnake(key);
      snakeCaseObj[snakeKey] = msg[key];
    }
    return btoa(JSON.stringify(snakeCaseObj));
  }
}

/**
 * Deserialize a JSON string to a `Message`.
 */
export function deserialize(base64Str: string): Message {
  const parsed = JSON.parse(atob(base64Str));
  if (typeof parsed === "string" || typeof parsed === "number") {
    return parsed;
  } else {
    let camelCaseObj = {} as { [key: string]: Message | string | number };
    for (const key of Object.keys(parsed)) {
      const camelKey = snakeToCamel(key);
      camelCaseObj[camelKey] = parsed[key];
    }
    return camelCaseObj;
  }
}

/**
 * Convert a string from `camelCase` to `snake_case`.
 */
function camelToSnake(str: string): string {
  return str.replace(/([A-Z])/g, "_$1").toLowerCase();
}

/**
 * Convert a string from `snake_case` to `camelCase`.
 */
function snakeToCamel(str: string): string {
  return str.replace(/(_[a-z])/g, (group) => group.toUpperCase().replace('_', ''));
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
