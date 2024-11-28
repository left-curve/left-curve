import type { Json, JsonValue } from "@left-curve/types";
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
  payload: Json | JsonValue,
  transformFn: (str: string) => string,
): Json | JsonValue {
  // for strings, numbers, and nulls, there's no key to be transformed
  if (typeof payload !== "object" || payload === null) {
    return payload;
  }

  // for arrays, we recursively transform each element
  if (Array.isArray(payload)) {
    return payload.map((element) => recursiveTransform(element, transformFn));
  }

  // for objects, we go through each key, transforming it to snake_case
  const obj = Object.create({});
  for (const [key, value] of Object.entries(payload)) {
    obj[transformFn(key)] = recursiveTransform(value, transformFn);
  }
  return obj;
}

/**
 * Given a value, run a transform function if the value is defined.
 * If the value is undefined, return undefined.
 * @param transform The transform function to run.
 * @param value The value to transform.
 * @returns The transformed value or undefined.
 */
export function mayTransform<T, U>(
  transform: (val: T) => U,
  value: T | null | undefined,
): U | undefined {
  return value === undefined || value === null ? undefined : transform(value);
}
