import { describe, expect, it } from "vitest";

import { decodeHex } from "../encoding/hex.js";
import { Ripemd160 } from "./ripemd.js";

describe("ripemd160", () => {
  it("constructor initializes correctly", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Ripemd160(data);
    expect(hash).toBeInstanceOf(Ripemd160);
  });

  it("Ripemd160 update method works correctly", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Ripemd160();
    hash.update(data);
    expect(hash.update(data)).toBe(hash);
  });

  it("Ripemd160 method works correctly", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Ripemd160(data);
    const digest = hash.digest();
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(20);
  });

  it("works for empty input", () => {
    {
      const hash = new Ripemd160(new Uint8Array([])).digest();
      expect(hash).toEqual(decodeHex("9c1185a5c5e9fc54612808977ee8f548b2258d31"));
    }
    {
      const hash = new Ripemd160().digest();
      expect(hash).toEqual(decodeHex("9c1185a5c5e9fc54612808977ee8f548b2258d31"));
    }
  });
});
