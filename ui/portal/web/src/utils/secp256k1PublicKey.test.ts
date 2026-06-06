import { describe, expect, it } from "vitest";
import { Secp256k1 } from "@left-curve/crypto";
import { encodeBase64, encodeHex } from "@left-curve/encoding";

import { parseSecp256k1PublicKey } from "./secp256k1PublicKey";

const privateKey = Uint8Array.from({ length: 32 }, (_, index) => index + 1);
const keyPair = new Secp256k1(privateKey);
const compressedPublicKey = keyPair.getPublicKey(true);
const uncompressedPublicKey = keyPair.getPublicKey(false);
const compressedBase64 = encodeBase64(compressedPublicKey);

describe("parseSecp256k1PublicKey", () => {
  it("parses compressed hex public keys", () => {
    const parsed = parseSecp256k1PublicKey(`0x${encodeHex(compressedPublicKey)}`);

    expect(parsed?.publicKey).toEqual(compressedPublicKey);
    expect(parsed?.publicKeyBase64).toBe(compressedBase64);
  });

  it("parses uppercase-prefixed hex public keys", () => {
    const parsed = parseSecp256k1PublicKey(`0X${encodeHex(compressedPublicKey)}`);

    expect(parsed?.publicKeyBase64).toBe(compressedBase64);
  });

  it("parses uncompressed hex public keys and normalizes them to compressed base64", () => {
    const parsed = parseSecp256k1PublicKey(encodeHex(uncompressedPublicKey));

    expect(parsed?.publicKey).toEqual(compressedPublicKey);
    expect(parsed?.publicKeyBase64).toBe(compressedBase64);
  });

  it("parses base64 public keys with whitespace", () => {
    const wrappedBase64 = `${compressedBase64.slice(0, 20)}\n${compressedBase64.slice(20)}`;
    const parsed = parseSecp256k1PublicKey(wrappedBase64);

    expect(parsed?.publicKey).toEqual(compressedPublicKey);
    expect(parsed?.publicKeyBase64).toBe(compressedBase64);
  });

  it("rejects invalid secp256k1 public keys", () => {
    const invalidCompressedPublicKey = Uint8Array.from({ length: 33 }, (_, index) => index + 1);

    expect(parseSecp256k1PublicKey("0x1234")).toBeNull();
    expect(parseSecp256k1PublicKey(encodeBase64(privateKey))).toBeNull();
    expect(parseSecp256k1PublicKey(encodeBase64(invalidCompressedPublicKey))).toBeNull();
    expect(parseSecp256k1PublicKey("not a key")).toBeNull();
  });
});
