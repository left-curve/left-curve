import { describe, expect, it } from "vitest";
import { decodeEndian32, encodeEndian32 } from "./endian32.js";

describe("encodeEndian32", () => {
  describe("big-endian", () => {
    it("should encode a number as 32-bit big-endian bytes", () => {
      const encoded = encodeEndian32(305419896); // 0x12345678
      expect(encoded).toEqual(new Uint8Array([0x12, 0x34, 0x56, 0x78]));
    });

    it("should encode the minimum 32-bit integer value", () => {
      const encoded = encodeEndian32(0); // 0x00000000
      expect(encoded).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00]));
    });

    it("should encode the maximum 32-bit integer value", () => {
      const encoded = encodeEndian32(4294967295); // 0xFFFFFFFF
      expect(encoded).toEqual(new Uint8Array([0xff, 0xff, 0xff, 0xff]));
    });
  });
});

describe("decodeEndian32", () => {
  describe("big-endian", () => {
    it("should decode 32-bit big-endian bytes into a number", () => {
      const bytes = new Uint8Array([0x12, 0x34, 0x56, 0x78]);
      const decoded = decodeEndian32(bytes);
      expect(decoded).toBe(305419896); // 0x12345678
    });

    it("should decode the minimum 32-bit integer value", () => {
      const bytes = new Uint8Array([0x00, 0x00, 0x00, 0x00]);
      const decoded = decodeEndian32(bytes);
      expect(decoded).toBe(0); // 0x00000000
    });

    it("should decode the maximum 32-bit integer value", () => {
      const bytes = new Uint8Array([0xff, 0xff, 0xff, 0xff]);
      const decoded = decodeEndian32(bytes);
      expect(decoded).toBe(4294967295); // 0xFFFFFFFF
    });

    it("should throw an error if byte array is not 4 bytes in length", () => {
      expect(() => decodeEndian32(new Uint8Array([0x12, 0x34, 0x56]))).toThrow(
        "expecting exactly 4 bytes, got 3",
      );
      expect(() => decodeEndian32(new Uint8Array([0x12, 0x34, 0x56, 0x78, 0x90]))).toThrow(
        "expecting exactly 4 bytes, got 5",
      );
    });
  });
});
