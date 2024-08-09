/**
 * Represents either an JSON object, an array, a string, a number, or a boolean.
 * Note that we utilize a recursive type definition here.
 */
export type Payload = { [key: string]: Payload } | Payload[] | string | number | boolean;

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
  payload: Payload,
  transformFn: (str: string) => string,
): Payload {
  // for strings, numbers, and nulls, there's no key to be transformed
  if (typeof payload !== "object" || payload === null) {
    return payload;
  }

  // for arrays, we recursively transform each element
  if (Array.isArray(payload)) {
    return payload.map((element) => recursiveTransform(element, transformFn));
  }

  // for objects, we go through each key, transforming it to snake_case
  const newObj = {} as { [key: string]: Payload };
  for (const [key, value] of Object.entries(payload)) {
    newObj[transformFn(key)] = recursiveTransform(value, transformFn);
  }
  return newObj;
}
