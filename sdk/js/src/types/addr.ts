import { decodeHex, encodeHex } from "..";

export class Addr {
  public bytes: Uint8Array;

  /**
   * Create a new `Addr` instance from the given byte array, which must be
   * 32 bytes in length.
   */
  public constructor(bytes: Uint8Array) {
    if (bytes.length !== 32) {
      throw new Error("address is not exactly 32 bytes");
    }
    this.bytes = bytes;
  }

  /**
   * Create a new `Addr` instance from a 32-byte, lowercase, 0x-prefixed hex
   * string.
   */
  public static fromString(str: string): Addr {
    // addresses must use the 0x prefix
    if (!str.startsWith("0x")) {
      throw new Error("address is not prefixed with `0x`");
    }
    // strip the 0x prefix
    str = str.substring(2);
    // reject uppercase hex strings
    if (!/^[0-9a-f]+$/.test(str)) {
      throw new Error("address contains non-hex or uppercase characters");
    }
    return new Addr(decodeHex(str));
  }

  /**
   * Stringify the `Addr`.
   */
  public toString(): string {
    return "0x" + encodeHex(this.bytes);
  }

  /**
   * Implementation for `JSON.parse`.
   */
  public static fromJSON(json: string): Addr {
    return JSON.parse(json, (_, value) => {
      if (typeof value === "string") {
        return Addr.fromString(value);
      }
      return value;
    });
  }

  /**
   * Implementation for `JSON.stringify`.
   */
  public toJSON(): string {
    return this.toString();
  }
}
