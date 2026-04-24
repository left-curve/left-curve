import type { Json, JsonValue } from "../types/index.js";
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
export function recursiveTransform<T extends Json | JsonValue = Json | JsonValue>(
  payload: T,
  transformFn: (str: string) => string,
): T {
  // for strings, numbers, and nulls, there's no key to be transformed
  if (typeof payload !== "object" || payload === null) {
    return payload as T;
  }

  // for arrays, we recursively transform each element
  if (Array.isArray(payload)) {
    return payload.map((element) => recursiveTransform(element, transformFn)) as T;
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

/**
 * Given an object, sort the keys and return a new object.
 * @param obj The object to sort.
 * @returns The sorted object.
 */
export function sortObject<T extends Json | JsonValue>(obj: T): T {
  if (typeof obj !== "object" || obj === null) {
    return obj;
  }

  const sorted = Object.create({});

  for (const key of Object.keys(obj).sort()) {
    sorted[key] = (obj as Json)[key];
  }

  return sorted;
}

/**
 * Invert the keys and values of an object.
 * @param obj The object to invert.
 * @returns The inverted object.
 */
export function invertObject(obj: object): object {
  if (typeof obj !== "object" || obj === null) {
    return obj;
  }

  function invert(obj: object) {
    return Object.entries(obj).reduce((acc, [key, value]) => {
      if (typeof value === "object") {
        invert(acc);
      } else {
        acc[value] = key;
      }
      return acc;
    }, Object.create({}));
  }

  return invert(obj);
}

/**
 * Given an object, return a new object with only the keys that have non-null and non-undefined values.
 * This function works recursively on nested objects.
 * @param obj The object to transform.
 * @returns The transformed object.
 */
export function plainObject<T = object>(obj: object, newObj = Object.create({})): T {
  return Object.entries(obj).reduce((acc, [key, value]) => {
    if (value === null || value === undefined) return acc;
    if (typeof value === "object" && !Array.isArray(value)) {
      plainObject(value, acc);
    } else {
      acc[key] = value;
    }
    return acc;
  }, newObj);
}
