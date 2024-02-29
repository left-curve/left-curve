import { decodeBase64, encodeBase64 } from "..";

export class Binary {
  public bytes: Uint8Array;

  /**
   * Create a new `Binary` instance from the given byte array.
   */
  public constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  /**
   * Create a new `Binary` instance from a base64-encoded string.
   */
  public static fromBase64(base64Str: string): Binary {
    return new Binary(decodeBase64(base64Str));
  }

  /**
   * Encode the `Binary` instance to a string in base64 encoding.
   */
  public toBase64(): string {
    return encodeBase64(this.bytes);
  }

  /**
   * Implementation for `JSON.parse`.
   */
  public static fromJSON(json: string): Binary {
    return JSON.parse(json, (_, value) => {
      if (typeof value === "string") {
        return Binary.fromBase64(value);
      }
      return value;
    });
  }

  /**
   * Implementation for `JSON.stringify`.
   */
  public toJSON(): string {
    return this.toBase64();
  }
}
