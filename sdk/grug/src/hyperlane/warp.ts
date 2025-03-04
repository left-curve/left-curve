import { encodeUint } from "../encoding/uint.js";
import type { Encoder } from "../types/encoding.js";
import type { Addr32 } from "./addr32.js";

export class TokenMessage implements Encoder {
  declare recipient: Addr32;
  declare amount: string;
  declare metadata: Uint8Array;

  private constructor(params: Omit<TokenMessage, "encode">) {
    this.recipient = params.recipient;
    this.amount = params.amount;
    this.metadata = params.metadata;
  }

  static from(params: Omit<TokenMessage, "encode">) {
    return new TokenMessage(params);
  }

  encode(): Uint8Array {
    let offset = 0;
    const buf = new Uint8Array(64 + this.metadata.byteLength);
    buf.set(this.recipient.encode(), offset);
    offset += 32;
    buf.set(encodeUint(this.amount, 32), offset);
    offset += 32;
    buf.set(this.metadata, offset);
    return buf;
  }
}
