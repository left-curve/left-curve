import superjson from "superjson";
import type { JsonValue } from "../types/encoding.js";
import { recursiveTransform } from "../utils/mappers.js";
import { camelToSnake, snakeToCamel } from "../utils/strings.js";
import { decodeBase64, encodeBase64 } from "./base64.js";

superjson.registerCustom(
  {
    isApplicable: (v: Uint8Array): v is Uint8Array => v?.constructor === Uint8Array,
    serialize: encodeBase64,
    deserialize: decodeBase64,
  },
  "Uint8Array",
);

export function serializeJson<T>(value: T): string {
  return superjson.stringify(value);
}

export function deserializeJson<T>(value: string): T {
  return superjson.parse<T>(value);
}

export function sortedObject(obj: JsonValue): JsonValue {
  if (typeof obj !== "object" || obj === null) {
    return obj;
  }
  if (Array.isArray(obj)) {
    return obj.map(sortedObject);
  }
  const sortedKeys = Object.keys(obj).sort();
  const result = Object.create({});
  sortedKeys.forEach((key) => {
    result[key] = sortedObject(obj[key]);
  });
  return result;
}

export function sortedJsonStringify(obj: JsonValue): string {
  return JSON.stringify(sortedObject(obj));
}

export function snakeCaseJsonSerialization(obj: JsonValue): string {
  return JSON.stringify(recursiveTransform(obj, camelToSnake));
}

export function camelCaseJsonDeserialization<T = JsonValue>(s: string): T {
  return recursiveTransform(JSON.parse(s), snakeToCamel);
}
