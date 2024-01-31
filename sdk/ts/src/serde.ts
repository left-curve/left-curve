/**
 * Represents either an JSON object, a string, or a number.
 * Note that we utilize a recursive type definition here.
 */
export type Payload = { [key: string]: Payload } | string | number;

/**
 * Serialize a payload to JSON string.
 * The payload should use camelCase, while the JSON string would have snale_case.
 */
export function serialize(msg: Payload): string {
  if (typeof msg === "string" || typeof msg === "number") {
    return JSON.stringify(msg);
  } else {
    let snakeCaseObj = {} as { [key: string]: Payload };
    for (const key of Object.keys(msg)) {
      const snakeKey = camelToSnake(key);
      snakeCaseObj[snakeKey] = msg[key];
    }
    return JSON.stringify(snakeCaseObj);
  }
}

/**
 * Deserialize a JSON string to a payload.
 * The JSON string should use snake_case, while the payload would have camelCase.
 */
export function deserialize(base64Str: string): Payload {
  const parsed = JSON.parse(base64Str);
  if (typeof parsed === "string" || typeof parsed === "number") {
    return parsed;
  } else {
    let camelCaseObj = {} as { [key: string]: Payload };
    for (const key of Object.keys(parsed)) {
      const camelKey = snakeToCamel(key);
      camelCaseObj[camelKey] = parsed[key];
    }
    return camelCaseObj;
  }
}

/**
 * Convert a string from camelCase to snake_case.
 */
function camelToSnake(str: string): string {
  return str.replace(/([A-Z])/g, "_$1").toLowerCase();
}

/**
 * Convert a string from snake_case to camelCase.
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
