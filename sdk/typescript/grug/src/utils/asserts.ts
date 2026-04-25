/**
 * A runtime checker that ensures a given value is set (i.e. not undefined or null)
 *
 * This is used when you want to verify that data at runtime matches the expected type.
 */
export function assertSet<T>(value: T): T {
  if ((value as unknown) === undefined) {
    throw new Error("Value must not be undefined");
  }

  if ((value as unknown) === null) {
    throw new Error("Value must not be null");
  }

  return value;
}

/**
 * A runtime checker that ensures a given value is a boolean
 *
 * This is used when you want to verify that data at runtime matches the expected type.
 * This implies assertSet.
 */
export function assertBoolean(value: boolean): boolean {
  assertSet(value);
  if (typeof (value as unknown) !== "boolean") {
    throw new Error("Value must be a boolean");
  }
  return value;
}

/**
 * A runtime checker that ensures a given value is a string.
 *
 * This is used when you want to verify that data at runtime matches the expected type.
 * This implies assertSet.
 */
export function assertString(value: string): string {
  assertSet(value);
  if (typeof (value as unknown) !== "string") {
    throw new Error("Value must be a string");
  }
  return value;
}

/**
 * A runtime checker that ensures a given value is a number
 *
 * This is used when you want to verify that data at runtime matches the expected type.
 * This implies assertSet.
 */
export function assertNumber(value: number): number {
  assertSet(value);
  if (typeof (value as unknown) !== "number") {
    throw new Error("Value must be a number");
  }
  return value;
}

/**
 * A runtime checker that ensures a given value is an array
 *
 * This is used when you want to verify that data at runtime matches the expected type.
 * This implies assertSet.
 */
export function assertArray<T>(value: readonly T[]): readonly T[] {
  assertSet(value);
  if (!Array.isArray(value as unknown)) {
    throw new Error("Value must be a an array");
  }
  return value;
}

/**
 * A runtime checker that ensures a given value is an object in the sense of JSON
 * (an unordered collection of key–value pairs where the keys are strings)
 *
 * This is used when you want to verify that data at runtime matches the expected type.
 * This implies assertSet.
 */
export function assertObject<T>(value: T): T {
  assertSet(value);
  if (typeof (value as unknown) !== "object") {
    throw new Error("Value must be an object");
  }

  // Exclude special kind of objects like Array, Date or Uint8Array
  // Object.prototype.toString() returns a specified value:
  // http://www.ecma-international.org/ecma-262/7.0/index.html#sec-object.prototype.tostring
  if (Object.prototype.toString.call(value) !== "[object Object]") {
    throw new Error("Value must be a simple object");
  }

  return value;
}

/**
 * Throws an error if value matches the empty value for the
 * given type (array/string of length 0, number of value 0, ...)
 *
 * Otherwise returns the value.
 *
 * This implies assertSet
 */
export function assertNotEmpty<T>(value: T): T {
  assertSet(value);

  if (typeof value === "number" && value === 0) {
    throw new Error("must provide a non-zero value");
  }
  if ((value as ArrayLike<unknown>).length === 0) {
    throw new Error("must provide a non-empty value");
  }
  return value;
}

/**
 * A deep equality checker that works for objects, arrays, and primitives.
 * It is based on the fast-deep-equal package, but with some modifications.
 * @param a The first value to compare.
 * @param b The second value to compare.
 * @returns true if the values are deeply equal, false otherwise.
 * Forked from https://github.com/epoberezkin/fast-deep-equal
 */
export function assertDeepEqual(a: any, b: any) {
  if (a === b) return true;

  if (a && b && typeof a === "object" && typeof b === "object") {
    if (a.constructor !== b.constructor) return false;

    let length: number;
    let i: number;

    if (Array.isArray(a) && Array.isArray(b)) {
      length = a.length;
      if (length !== b.length) return false;
      for (i = length; i-- !== 0; ) if (!assertDeepEqual(a[i], b[i])) return false;
      return true;
    }

    if (a.valueOf !== Object.prototype.valueOf) return a.valueOf() === b.valueOf();
    if (a.toString !== Object.prototype.toString) return a.toString() === b.toString();

    const keys = Object.keys(a);
    length = keys.length;
    if (length !== Object.keys(b).length) return false;

    for (i = length; i-- !== 0; ) if (!Object.hasOwn(b, keys[i]!)) return false;

    for (i = length; i-- !== 0; ) {
      const key = keys[i];

      if (key && !assertDeepEqual(a[key], b[key])) return false;
    }

    return true;
  }

  // true if both NaN, false otherwise
  // biome-ignore lint/suspicious/noSelfCompare: it uses self-comparison to check for NaN
  return a !== a && b !== b;
}
