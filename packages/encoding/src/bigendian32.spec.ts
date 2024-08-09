import { describe, expect, it } from "vitest";
import { decodeBigEndian32, encodeBigEndian32 } from "./bigendian32";

describe("encodeBigEndian32", () => {
  it("should encode a number as 32-bit big-endian bytes", () => {
    const encoded = encodeBigEndian32(305419896); // 0x12345678
    expect(encoded).toEqual(new Uint8Array([0x12, 0x34, 0x56, 0x78]));
  });

  it("should encode the minimum 32-bit integer value", () => {
    const encoded = encodeBigEndian32(0); // 0x00000000
    expect(encoded).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00]));
  });

  it("should encode the maximum 32-bit integer value", () => {
    const encoded = encodeBigEndian32(4294967295); // 0xFFFFFFFF
    expect(encoded).toEqual(new Uint8Array([0xff, 0xff, 0xff, 0xff]));
  });
});

describe("decodeBigEndian32", () => {
  it("should decode 32-bit big-endian bytes into a number", () => {
    const bytes = new Uint8Array([0x12, 0x34, 0x56, 0x78]);
    const decoded = decodeBigEndian32(bytes);
    expect(decoded).toBe(305419896); // 0x12345678
  });

  it("should decode the minimum 32-bit integer value", () => {
    const bytes = new Uint8Array([0x00, 0x00, 0x00, 0x00]);
    const decoded = decodeBigEndian32(bytes);
    expect(decoded).toBe(0); // 0x00000000
  });

  it("should decode the maximum 32-bit integer value", () => {
    const bytes = new Uint8Array([0xff, 0xff, 0xff, 0xff]);
    const decoded = decodeBigEndian32(bytes);
    expect(decoded).toBe(4294967295); // 0xFFFFFFFF
  });

  it("should throw an error if byte array is not 4 bytes in length", () => {
    expect(() => decodeBigEndian32(new Uint8Array([0x12, 0x34, 0x56]))).toThrow(
      "expecting exactly 4 bytes, got 3",
    );
    expect(() => decodeBigEndian32(new Uint8Array([0x12, 0x34, 0x56, 0x78, 0x90]))).toThrow(
      "expecting exactly 4 bytes, got 5",
    );
  });
});
