import { describe, expect, it } from "vitest";
import { decodeUtf8, encodeUtf8 } from "./utf8";

describe("utf8", () => {
  it("encodes ascii strings", () => {
    expect(encodeUtf8("")).toEqual(new Uint8Array([]));
    expect(encodeUtf8("abc")).toEqual(new Uint8Array([0x61, 0x62, 0x63]));
    expect(encodeUtf8(" ?=-n|~+-*/\\")).toEqual(
      new Uint8Array([0x20, 0x3f, 0x3d, 0x2d, 0x6e, 0x7c, 0x7e, 0x2b, 0x2d, 0x2a, 0x2f, 0x5c]),
    );
  });

  it("decodes ascii string", () => {
    expect(decodeUtf8(new Uint8Array([]))).toEqual("");
    expect(decodeUtf8(new Uint8Array([0x61, 0x62, 0x63]))).toEqual("abc");
    expect(
      decodeUtf8(
        new Uint8Array([0x20, 0x3f, 0x3d, 0x2d, 0x6e, 0x7c, 0x7e, 0x2b, 0x2d, 0x2a, 0x2f, 0x5c]),
      ),
    ).toEqual(" ?=-n|~+-*/\\");
  });

  it("encodes null character", () => {
    expect(encodeUtf8("\u0000")).toEqual(new Uint8Array([0x00]));
  });

  it("decodes null byte", () => {
    expect(decodeUtf8(new Uint8Array([0x00]))).toEqual("\u0000");
  });

  it("encodes Basic Multilingual Plane strings", () => {
    expect(encodeUtf8("ö")).toEqual(new Uint8Array([0xc3, 0xb6]));
    expect(encodeUtf8("¥")).toEqual(new Uint8Array([0xc2, 0xa5]));
    expect(encodeUtf8("Ф")).toEqual(new Uint8Array([0xd0, 0xa4]));
    expect(encodeUtf8("ⱴ")).toEqual(new Uint8Array([0xe2, 0xb1, 0xb4]));
    expect(encodeUtf8("ⵘ")).toEqual(new Uint8Array([0xe2, 0xb5, 0x98]));
  });

  it("decodes Basic Multilingual Plane strings", () => {
    expect(decodeUtf8(new Uint8Array([0xc3, 0xb6]))).toEqual("ö");
    expect(decodeUtf8(new Uint8Array([0xc2, 0xa5]))).toEqual("¥");
    expect(decodeUtf8(new Uint8Array([0xd0, 0xa4]))).toEqual("Ф");
    expect(decodeUtf8(new Uint8Array([0xe2, 0xb1, 0xb4]))).toEqual("ⱴ");
    expect(decodeUtf8(new Uint8Array([0xe2, 0xb5, 0x98]))).toEqual("ⵘ");
  });

  it("encodes Supplementary Multilingual Plane strings", () => {
    // U+1F0A1
    expect(encodeUtf8("🂡")).toEqual(new Uint8Array([0xf0, 0x9f, 0x82, 0xa1]));
    // U+1034A
    expect(encodeUtf8("𐍊")).toEqual(new Uint8Array([0xf0, 0x90, 0x8d, 0x8a]));
  });

  it("decodes Supplementary Multilingual Plane strings", () => {
    // U+1F0A1
    expect(decodeUtf8(new Uint8Array([0xf0, 0x9f, 0x82, 0xa1]))).toEqual("🂡");
    // U+1034A
    expect(decodeUtf8(new Uint8Array([0xf0, 0x90, 0x8d, 0x8a]))).toEqual("𐍊");
  });

  it("throws on invalid utf8 bytes", () => {
    // Broken UTF8 example from https://github.com/nodejs/node/issues/16894
    expect(() => decodeUtf8(new Uint8Array([0xf0, 0x80, 0x80]))).toThrow();
  });

  describe("decodeUtf8", () => {
    it("replaces characters in lossy mode", () => {
      expect(decodeUtf8(new Uint8Array([]), true)).toEqual("");
      expect(decodeUtf8(new Uint8Array([0x61, 0x62, 0x63]), true)).toEqual("abc");
    });
  });
});
