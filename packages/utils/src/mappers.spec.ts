import { describe, expect, it } from "vitest";
import { recursiveTransform } from "./mappers";
import { camelToSnake, snakeToCamel } from "./strings";

describe("recursiveTransform", () => {
  it("transforms keys in objects from camelCase to snake_case", () => {
    const payload = {
      someKey: "value",
      anotherKey: { nestedKey: "nestedValue" },
    };
    const expected = {
      some_key: "value",
      another_key: { nested_key: "nestedValue" },
    };
    expect(recursiveTransform(payload, camelToSnake)).toEqual(expected);
  });

  it("transforms keys in objects from snake_case to camelCase", () => {
    const payload = {
      some_key: "value",
      another_key: { nested_key: "nestedValue" },
    };
    const expected = {
      someKey: "value",
      anotherKey: { nestedKey: "nestedValue" },
    };
    expect(recursiveTransform(payload, snakeToCamel)).toEqual(expected);
  });

  it("transforms elements in arrays", () => {
    const payload = [{ someKey: "value" }, { anotherKey: "anotherValue" }];
    const expected = [{ some_key: "value" }, { another_key: "anotherValue" }];
    expect(recursiveTransform(payload, camelToSnake)).toEqual(expected);
  });

  it("handles strings, numbers, and booleans without transformation", () => {
    expect(recursiveTransform("string", camelToSnake)).toEqual("string");
    expect(recursiveTransform(123, camelToSnake)).toEqual(123);
    expect(recursiveTransform(true, camelToSnake)).toEqual(true);
  });

  it("handles null values without transformation", () => {
    expect(recursiveTransform(null!, camelToSnake)).toEqual(null);
  });

  it("handles empty objects and arrays", () => {
    expect(recursiveTransform({}, camelToSnake)).toEqual({});
    expect(recursiveTransform([], camelToSnake)).toEqual([]);
  });

  it("transforms nested arrays and objects", () => {
    const payload = { someKey: [{ nestedKey: "value" }] };
    const expected = { some_key: [{ nested_key: "value" }] };
    expect(recursiveTransform(payload, camelToSnake)).toEqual(expected);
  });

  it("transforms mixed types within arrays", () => {
    const payload = [{ someKey: "value" }, "string", 123, true];
    const expected = [{ some_key: "value" }, "string", 123, true];
    expect(recursiveTransform(payload, camelToSnake)).toEqual(expected);
  });

  it("transforms deeply nested structures", () => {
    const payload = { someKey: { nestedKey: { deepKey: "value" } } };
    const expected = { some_key: { nested_key: { deep_key: "value" } } };
    expect(recursiveTransform(payload, camelToSnake)).toEqual(expected);
  });
});
