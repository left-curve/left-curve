import { describe, expect, it } from "vitest";
import { decodeHex, encodeHex } from "./hex";

describe("Hex", () => {
  it("decode to hex", () => {
    // simple
    expect(decodeHex("")).toEqual(new Uint8Array([]));
    expect(decodeHex("00")).toEqual(new Uint8Array([0x00]));
    expect(decodeHex("01")).toEqual(new Uint8Array([0x01]));
    expect(decodeHex("10")).toEqual(new Uint8Array([0x10]));
    expect(decodeHex("11")).toEqual(new Uint8Array([0x11]));
    expect(decodeHex("112233")).toEqual(new Uint8Array([0x11, 0x22, 0x33]));
    expect(decodeHex("0123456789abcdef")).toEqual(
      new Uint8Array([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
    );

    // capital letters
    expect(decodeHex("AA")).toEqual(new Uint8Array([0xaa]));
    expect(decodeHex("aAbBcCdDeEfF")).toEqual(new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));

    // error
    expect(() => decodeHex("a")).toThrow();
    expect(() => decodeHex("aaa")).toThrow();
    expect(() => decodeHex("a!")).toThrow();
    expect(() => decodeHex("a ")).toThrow();
    expect(() => decodeHex("aa ")).toThrow();
    expect(() => decodeHex(" aa")).toThrow();
    expect(() => decodeHex("a a")).toThrow();
    expect(() => decodeHex("gg")).toThrow();
  });

  it("encode to hex", () => {
    expect(encodeHex(new Uint8Array([]))).toEqual("");
    expect(encodeHex(new Uint8Array([0x00]))).toEqual("00");
    expect(encodeHex(new Uint8Array([0x01]))).toEqual("01");
    expect(encodeHex(new Uint8Array([0x10]))).toEqual("10");
    expect(encodeHex(new Uint8Array([0x11]))).toEqual("11");
    expect(encodeHex(new Uint8Array([0x11, 0x22, 0x33]))).toEqual("112233");
    expect(encodeHex(new Uint8Array([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]))).toEqual(
      "0123456789abcdef",
    );
  });
});
