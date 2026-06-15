import { encodeBase64, encodeHex } from "@left-curve/encoding";
import { describe, expect, it } from "vitest";

import {
  Secp256k1,
  secp256k1NormalizePubKey,
  secp256k1ParsePubKey,
} from "./secp256k1.js";

const privateKey = Uint8Array.from({ length: 32 }, (_, index) => index + 1);
const keyPair = new Secp256k1(privateKey);
const compressedPublicKey = keyPair.getPublicKey(true);
const uncompressedPublicKey = keyPair.getPublicKey(false);
const compressedBase64 = encodeBase64(compressedPublicKey);

describe("secp256k1ParsePubKey", () => {
  it("parses compressed hex public keys", () => {
    expect(secp256k1ParsePubKey(`0x${encodeHex(compressedPublicKey)}`)).toEqual(
      compressedPublicKey,
    );
  });

  it("parses uppercase-prefixed hex public keys", () => {
    expect(secp256k1ParsePubKey(`0X${encodeHex(compressedPublicKey)}`)).toEqual(
      compressedPublicKey,
    );
  });

  it("parses compressed hex public keys without a prefix", () => {
    expect(secp256k1ParsePubKey(encodeHex(compressedPublicKey))).toEqual(compressedPublicKey);
  });

  it("parses uncompressed hex public keys and normalizes them to compressed bytes", () => {
    expect(secp256k1ParsePubKey(encodeHex(uncompressedPublicKey))).toEqual(compressedPublicKey);
  });

  it("parses base64 public keys with whitespace", () => {
    const wrappedBase64 = `${compressedBase64.slice(0, 20)}\n${compressedBase64.slice(20)}`;

    expect(secp256k1ParsePubKey(wrappedBase64)).toEqual(compressedPublicKey);
  });

  it("parses explicitly padded base64 public keys", () => {
    expect(secp256k1ParsePubKey(encodeBase64(uncompressedPublicKey))).toEqual(compressedPublicKey);
  });

  it("returns null for empty or invalid public keys", () => {
    const invalidCompressedPublicKey = Uint8Array.from({ length: 33 }, (_, index) => index + 1);

    expect(secp256k1ParsePubKey("")).toBeNull();
    expect(secp256k1ParsePubKey(" \n\t ")).toBeNull();
    expect(secp256k1ParsePubKey("0x1234")).toBeNull();
    expect(secp256k1ParsePubKey(`${encodeHex(compressedPublicKey)}f`)).toBeNull();
    expect(secp256k1ParsePubKey(encodeBase64(privateKey))).toBeNull();
    expect(secp256k1ParsePubKey(encodeBase64(invalidCompressedPublicKey))).toBeNull();
    expect(secp256k1ParsePubKey("not a key")).toBeNull();
  });
});

describe("secp256k1NormalizePubKey", () => {
  it("normalizes uncompressed public keys to compressed bytes", () => {
    expect(secp256k1NormalizePubKey(uncompressedPublicKey)).toEqual(compressedPublicKey);
  });
});
