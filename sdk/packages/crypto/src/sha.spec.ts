import { encodeHex } from "@left-curve/encoding";
import { describe, expect, it } from "vitest";
import { Sha256, Sha512, sha256, sha512 } from "./sha.js";

describe("Sha256", () => {
  it("should initialize with no data", () => {
    const hash = new Sha256();
    expect(hash).toBeInstanceOf(Sha256);
  });

  it("should initialize with data", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Sha256(data);
    expect(hash).toBeInstanceOf(Sha256);
  });

  it("should update hash with data", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Sha256();
    hash.update(data);
    expect(hash).toBeInstanceOf(Sha256);
  });

  it("should return correct digest", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Sha256(data);
    const digest = hash.digest();
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(32); // sha256 produces a 32-byte hash
  });

  it("should return correct digest for multiple updates", () => {
    const data1 = new Uint8Array([1, 2, 3, 4]);
    const data2 = new Uint8Array([5, 6, 7, 8]);
    const hash = new Sha256();
    hash.update(data1);
    hash.update(data2);
    const digest = hash.digest();
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(32); // sha256 produces a 32-byte hash
  });

  it("sha256 convenience function should return correct digest", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const digest = sha256(data);
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(32); // sha256 produces a 32-byte hash
  });

  it("works for empty input", () => {
    {
      const hash = new Sha256(new Uint8Array([])).digest();
      expect(encodeHex(hash)).toEqual(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      );
    }
    {
      const hash = new Sha256().digest();
      expect(encodeHex(hash)).toEqual(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      );
    }
  });
});

describe("Sha512", () => {
  it("should initialize with no data", () => {
    const hash = new Sha512();
    expect(hash).toBeInstanceOf(Sha512);
  });

  it("should initialize with data", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Sha512(data);
    expect(hash).toBeInstanceOf(Sha512);
  });

  it("should update hash with data", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Sha512();
    hash.update(data);
    expect(hash).toBeInstanceOf(Sha512);
  });

  it("should return correct digest", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const hash = new Sha512(data);
    const digest = hash.digest();
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(64); // sha512 produces a 64-byte hash
  });

  it("should return correct digest for multiple updates", () => {
    const data1 = new Uint8Array([1, 2, 3, 4]);
    const data2 = new Uint8Array([5, 6, 7, 8]);
    const hash = new Sha512();
    hash.update(data1);
    hash.update(data2);
    const digest = hash.digest();
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(64); // sha512 produces a 64-byte hash
  });

  it("sha512 convenience function should return correct digest", () => {
    const data = new Uint8Array([1, 2, 3, 4]);
    const digest = sha512(data);
    expect(digest).toBeInstanceOf(Uint8Array);
    expect(digest.length).toBe(64); // sha512 produces a 64-byte hash
  });

  it("works for empty input", () => {
    {
      const hash = new Sha512(new Uint8Array([])).digest();
      expect(encodeHex(hash)).toEqual(
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
      );
    }
    {
      const hash = new Sha512().digest();
      expect(encodeHex(hash)).toEqual(
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
      );
    }
  });
});
