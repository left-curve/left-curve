import { decodeHex, encodeHex } from "../encoding/hex.js";
import type { Address } from "../types/address.js";
import type { Encoder } from "../types/encoding.js";

export class Addr32 implements Encoder {
  #address: Uint8Array;
  private constructor(address: Uint8Array) {
    this.#address = address;
  }

  static from(address: Address) {
    const addr32 = new Uint8Array(32);
    addr32.set(decodeHex(address), 12);
    return new Addr32(addr32);
  }

  static decode(buf: Uint8Array) {
    return new Addr32(buf);
  }

  get address() {
    return encodeHex(this.#address);
  }

  encode() {
    return this.#address;
  }
}
