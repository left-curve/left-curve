import { describe, expect, it } from "vitest";
import { camelToSnake, snakeToCamel } from "./strings.js";

describe("camelToSnake", () => {
  it("converts simple camelCase to snake_case", () => {
    expect(camelToSnake("camelCase")).toEqual("camel_case");
  });

  it("converts camelCase with multiple uppercase letters to snake_case", () => {
    expect(camelToSnake("thisIsCamelCase")).toEqual("this_is_camel_case");
  });

  it("returns the same string if already in snake_case", () => {
    expect(camelToSnake("snake_case")).toEqual("snake_case");
  });

  it("returns an empty string when input is empty", () => {
    expect(camelToSnake("")).toEqual("");
  });

  it("should handle strings with numbers", () => {
    expect(camelToSnake("camelCase123Test")).toBe("camel_case123_test");
  });
});

describe("snakeToCamel", () => {
  it("converts simple snake_case to camelCase", () => {
    expect(snakeToCamel("snake_case")).toEqual("snakeCase");
  });

  it("converts snake_case with multiple underscores to camelCase", () => {
    expect(snakeToCamel("this_is_snake_case")).toEqual("thisIsSnakeCase");
  });

  it("returns the same string if already in camelCase", () => {
    expect(snakeToCamel("camelCase")).toEqual("camelCase");
  });

  it("returns an empty string when input is empty", () => {
    expect(snakeToCamel("")).toEqual("");
  });

  // TODO: Cover this test case in the function camelToSnake
  it.skip("should handle strings with numbers", () => {
    expect(camelToSnake("camel_case123_test")).toBe("camelCase123Test");
  });
});
