import { decodeHex, encodeHex } from "@left-curve/sdk/encoding";

import type { Address, Encoder } from "@left-curve/sdk/types";

export function toAddr32(address: `0x${string}`): `0x${string}` {
  return `0x${address.slice(2).padStart(64, "0")}` as `0x${string}`;
}

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
