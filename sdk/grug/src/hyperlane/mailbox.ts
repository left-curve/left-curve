import { encodeEndian32 } from "../encoding/endian32.js";
import type { Encoder } from "../types/encoding.js";
import type { Addr32 } from "./addr32.js";

export type Domain = number;

export const MAILBOX_VERSION = 3;
export const HYPERLANE_DOMAIN_KEY = "HYPERLANE";

export class Message implements Encoder {
  version: number;
  nonce: number;
  originDomain: Domain;
  sender: Addr32;
  destinationDomain: Domain;
  recipient: Addr32;
  body: Uint8Array;

  private constructor(params: Omit<Message, "encode" | "decode">) {
    this.version = params.version;
    this.nonce = params.nonce;
    this.originDomain = params.originDomain;
    this.sender = params.sender;
    this.destinationDomain = params.destinationDomain;
    this.recipient = params.recipient;
    this.body = params.body;
  }

  static from(params: Omit<Message, "encode" | "decode">) {
    return new Message(params);
  }

  encode() {
    let offset = 0;

    const buf = new Uint8Array(77 + this.body.byteLength);
    buf.set([this.version], offset);
    offset += 1;
    buf.set(encodeEndian32(this.nonce), offset);
    offset += 4;
    buf.set(encodeEndian32(this.originDomain), offset);
    offset += 4;
    buf.set(this.sender.encode(), offset);
    offset += 32;
    buf.set(encodeEndian32(this.destinationDomain), offset);
    offset += 4;
    buf.set(this.recipient.encode(), offset);
    offset += 32;
    buf.set(this.body, offset);

    return buf;
  }
}
