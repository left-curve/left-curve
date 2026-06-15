import type { Json, JsonValue } from "@left-curve/types";

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
 * Given a payload, recursively transform the case of the keys.
 */
export function recursiveTransform<T extends Json | JsonValue = Json | JsonValue>(
  payload: T,
  transformFn: (str: string) => string,
): T {
  if (typeof payload !== "object" || payload === null) {
    return payload as T;
  }

  if (Array.isArray(payload)) {
    return payload.map((element) => recursiveTransform(element, transformFn)) as T;
  }

  const obj = Object.create({});
  for (const [key, value] of Object.entries(payload)) {
    obj[transformFn(key)] = recursiveTransform(value, transformFn);
  }
  return obj;
}
