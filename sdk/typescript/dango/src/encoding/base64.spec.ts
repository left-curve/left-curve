import { describe, expect, it } from "vitest";
import {
  base64ToBase64Url,
  base64UrlToBase64,
  decodeBase64,
  decodeBase64Url,
  encodeBase64,
  encodeBase64Url,
} from "./base64.js";

describe("base64", () => {
  it("encodes to base64", () => {
    expect(encodeBase64(new Uint8Array([]))).toEqual("");
    expect(encodeBase64(new Uint8Array([0x00]))).toEqual("AA==");
    expect(encodeBase64(new Uint8Array([0x00, 0x00]))).toEqual("AAA=");
    expect(encodeBase64(new Uint8Array([0x00, 0x00, 0x00]))).toEqual("AAAA");
    expect(encodeBase64(new Uint8Array([0x00, 0x00, 0x00, 0x00]))).toEqual("AAAAAA==");
    expect(encodeBase64(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00]))).toEqual("AAAAAAA=");
    expect(encodeBase64(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]))).toEqual("AAAAAAAA");
    expect(encodeBase64(new Uint8Array([0x61]))).toEqual("YQ==");
    expect(encodeBase64(new Uint8Array([0x62]))).toEqual("Yg==");
    expect(encodeBase64(new Uint8Array([0x63]))).toEqual("Yw==");
    expect(encodeBase64(new Uint8Array([0x61, 0x62, 0x63]))).toEqual("YWJj");
  });

  it("decodes from base64", () => {
    expect(decodeBase64("")).toEqual(new Uint8Array([]));
    expect(decodeBase64("AA==")).toEqual(new Uint8Array([0x00]));
    expect(decodeBase64("AAA=")).toEqual(new Uint8Array([0x00, 0x00]));
    expect(decodeBase64("AAAA")).toEqual(new Uint8Array([0x00, 0x00, 0x00]));
    expect(decodeBase64("AAAAAA==")).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00]));
    expect(decodeBase64("AAAAAAA=")).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00]));
    expect(decodeBase64("AAAAAAAA")).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
    expect(decodeBase64("YQ==")).toEqual(new Uint8Array([0x61]));
    expect(decodeBase64("Yg==")).toEqual(new Uint8Array([0x62]));
    expect(decodeBase64("Yw==")).toEqual(new Uint8Array([0x63]));
    expect(decodeBase64("YWJj")).toEqual(new Uint8Array([0x61, 0x62, 0x63]));

    // invalid length
    expect(() => decodeBase64("a")).toThrow();
    expect(() => decodeBase64("aa")).toThrow();
    expect(() => decodeBase64("aaa")).toThrow();

    // proper length including invalid character
    expect(() => decodeBase64("aaa!")).toThrow();
    expect(() => decodeBase64("aaa*")).toThrow();
    expect(() => decodeBase64("aaaä")).toThrow();

    // proper length plus invalid character
    expect(() => decodeBase64("aaaa!")).toThrow();
    expect(() => decodeBase64("aaaa*")).toThrow();
    expect(() => decodeBase64("aaaaä")).toThrow();

    // extra spaces
    expect(() => decodeBase64("aaaa ")).toThrow();
    expect(() => decodeBase64(" aaaa")).toThrow();
    expect(() => decodeBase64("aa aa")).toThrow();
    expect(() => decodeBase64("aaaa\n")).toThrow();
    expect(() => decodeBase64("\naaaa")).toThrow();
    expect(() => decodeBase64("aa\naa")).toThrow();

    // position of =
    expect(() => decodeBase64("=aaa")).toThrow();
    expect(() => decodeBase64("==aa")).toThrow();

    expect(() => decodeBase64("AAA=AAA=")).toThrow();

    // wrong number of =
    expect(() => decodeBase64("a===")).toThrow();
  });
});

describe("base64Url", () => {
  it("converts base64url to base64", () => {
    expect(base64UrlToBase64("")).toEqual("");
    expect(base64UrlToBase64("SGVsbG8-29ybGQ_")).toEqual("SGVsbG8+29ybGQ/");
    expect(base64UrlToBase64("YWJjZA==")).toEqual("YWJjZA==");
    expect(base64UrlToBase64("YWJjZA")).toEqual("YWJjZA");
    expect(base64UrlToBase64("YWJjZA_")).toEqual("YWJjZA/");
    expect(base64UrlToBase64("YWJjZA-")).toEqual("YWJjZA+");
  });

  it("converts base64 to base64url", () => {
    expect(base64ToBase64Url("")).toEqual("");
    expect(base64ToBase64Url("SGVsbG8+29ybGQ/")).toEqual("SGVsbG8-29ybGQ_");
    expect(base64ToBase64Url("YWJjZA==")).toEqual("YWJjZA");
    expect(base64ToBase64Url("YWJjZA")).toEqual("YWJjZA");
    expect(base64ToBase64Url("YWJjZA/")).toEqual("YWJjZA_");
    expect(base64ToBase64Url("YWJjZA+")).toEqual("YWJjZA-");
  });

  it("decodes from base64Url", () => {
    expect(decodeBase64Url("")).toEqual(new Uint8Array([]));
    expect(decodeBase64Url("AA==")).toEqual(new Uint8Array([0x00]));
    expect(decodeBase64Url("AAA=")).toEqual(new Uint8Array([0x00, 0x00]));
    expect(decodeBase64Url("AAAA")).toEqual(new Uint8Array([0x00, 0x00, 0x00]));
    expect(decodeBase64Url("AAAAAA==")).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00]));
    expect(decodeBase64Url("AAAAAAA=")).toEqual(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00]));
    expect(decodeBase64Url("AAAAAAAA")).toEqual(
      new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    );
    expect(decodeBase64Url("YQ==")).toEqual(new Uint8Array([0x61]));
    expect(decodeBase64Url("Yg==")).toEqual(new Uint8Array([0x62]));
    expect(decodeBase64Url("Yw==")).toEqual(new Uint8Array([0x63]));
    expect(decodeBase64Url("YWJj")).toEqual(new Uint8Array([0x61, 0x62, 0x63]));

    // invalid length
    expect(() => decodeBase64Url("a")).toThrow();
    expect(() => decodeBase64Url("aa")).toThrow();
    expect(() => decodeBase64Url("aaa")).toThrow();

    // proper length including invalid character
    expect(() => decodeBase64Url("aaa!")).toThrow();
    expect(() => decodeBase64Url("aaa*")).toThrow();
    expect(() => decodeBase64Url("aaaä")).toThrow();

    // proper length plus invalid character
    expect(() => decodeBase64Url("aaaa!")).toThrow();
    expect(() => decodeBase64Url("aaaa*")).toThrow();
    expect(() => decodeBase64Url("aaaaä")).toThrow();

    // extra spaces
    expect(() => decodeBase64Url("aaaa ")).toThrow();
    expect(() => decodeBase64Url(" aaaa")).toThrow();
    expect(() => decodeBase64Url("aa aa")).toThrow();
    expect(() => decodeBase64Url("aaaa\n")).toThrow();
    expect(() => decodeBase64Url("\naaaa")).toThrow();
    expect(() => decodeBase64Url("aa\naa")).toThrow();

    // position of =
    expect(() => decodeBase64Url("=aaa")).toThrow();
    expect(() => decodeBase64Url("==aa")).toThrow();

    expect(() => decodeBase64Url("AAA=AAA=")).toThrow();
  });

  it("encodes to base64url", () => {
    expect(encodeBase64Url(new Uint8Array([]))).toEqual("");
    expect(encodeBase64Url(new Uint8Array([0x00]))).toEqual("AA");
    expect(encodeBase64Url(new Uint8Array([0x00, 0x00]))).toEqual("AAA");
    expect(encodeBase64Url(new Uint8Array([0x00, 0x00, 0x00]))).toEqual("AAAA");
    expect(encodeBase64Url(new Uint8Array([0x00, 0x00, 0x00, 0x00]))).toEqual("AAAAAA");
    expect(encodeBase64Url(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00]))).toEqual("AAAAAAA");
    expect(encodeBase64Url(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]))).toEqual(
      "AAAAAAAA",
    );
    expect(encodeBase64Url(new Uint8Array([0x61]))).toEqual("YQ");
    expect(encodeBase64Url(new Uint8Array([0x62]))).toEqual("Yg");
    expect(encodeBase64Url(new Uint8Array([0x63]))).toEqual("Yw");
    expect(encodeBase64Url(new Uint8Array([0x61, 0x62, 0x63]))).toEqual("YWJj");
  });
});
