import { decodeHex, encodeHex } from "..";

export class Hash {
  public bytes: Uint8Array;

  /**
   * Create a new `Hash` instance from the given byte array, which must be
   * 32 bytes in length.
   */
  public constructor(bytes: Uint8Array) {
    if (bytes.length !== 32) {
      throw new Error("hash is not exactly 32 bytes");
    }
    this.bytes = bytes;
  }

  /**
   * Create a new `Hash` instance from a 32-byte, lowercase hex string
   */
  public static fromHex(hexStr: string): Hash {
    // reject uppercase hex strings
    if (!/^[0-9a-f]+$/.test(hexStr)) {
      throw new Error("hash contains non-hex or uppercase characters");
    }
    return new Hash(decodeHex(hexStr));
  }

  /**
   * Stringify the `Hash` using the hex encoding.
   */
  public toHex(): string {
    return encodeHex(this.bytes);
  }

  /**
   * Implementation for `JSON.parse`.
   */
  public static parse(json: string): Hash {
    return JSON.parse(json, (_, value) => {
      if (typeof value === "string") {
        return Hash.fromHex(value);
      }
      return value;
    });
  }

  /**
   * Implementation for `JSON.stringify`.
   */
  public toJSON(): string {
    return this.toHex();
  }
}
