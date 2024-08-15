import { ripemd160, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";

import type { KeyPair } from "@leftcurve/crypto";
import type { AbstractSigner, Credential } from "@leftcurve/types";
import type { Message } from "@leftcurve/types";

export class PrivateKeySigner implements AbstractSigner {
  #keyPair: KeyPair;

  constructor(keyPair: KeyPair) {
    this.#keyPair = keyPair;
  }

  async getKeyId(): Promise<string> {
    return encodeHex(ripemd160(this.#keyPair.publicKey)).toUpperCase();
  }

  async signTx(msgs: Message[], chainId: string, sequence: number): Promise<Credential> {
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));
    const signature = await this.signBytes(tx);
    return { secp256k1: encodeBase64(signature) };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
