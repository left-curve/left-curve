import { encodeEndian32 } from "../encoding/endian32.js";
import type { Encoder } from "../types/encoding.js";
import type { Addr32 } from "./addr32.js";

export type ValidatorSet = {
  threshold: number;
  validators: Set<Addr32>;
};

export class Metadata implements Encoder {
  declare originMerkleTree: Addr32;
  declare merkleRoot: Uint8Array;
  declare merkleIndex: number;
  declare signatures: Uint8Array[];
  private constructor(params: Omit<Metadata, "encode">) {
    this.originMerkleTree = params.originMerkleTree;
    this.merkleRoot = params.merkleRoot;
    this.merkleIndex = params.merkleIndex;
    this.signatures = params.signatures;
  }

  static from(params: Omit<Metadata, "encode">): Metadata {
    return new Metadata(params);
  }

  encode(): Uint8Array {
    const buf = new Uint8Array(32 + 32 + 4 + this.signatures.length * 65);
    let offset = 0;
    buf.set(this.originMerkleTree.encode(), offset);
    offset += 32;
    buf.set(this.merkleRoot, offset);
    offset += 32;
    buf.set(encodeEndian32(this.merkleIndex), offset);
    offset += 4;
    this.signatures.forEach((signature) => {
      buf.set(signature, offset);
      offset += 65;
    });
    return buf;
  }
}
