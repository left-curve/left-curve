import { ripemd160 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex } from "@leftcurve/encoding";
import { createSignBytes } from "@leftcurve/types";

import type { KeyPair } from "@leftcurve/crypto";
import type { AbstractSigner } from "@leftcurve/types";
import type { Message, Tx } from "@leftcurve/types";

export class KeyPairSigner implements AbstractSigner<{ sequence: number }> {
  #keyPair: KeyPair;

  constructor(keyPair: KeyPair) {
    this.#keyPair = keyPair;
  }

  async getKeyId(): Promise<string> {
    return encodeHex(ripemd160(this.#keyPair.publicKey));
  }

  async signTx(
    msgs: Message[],
    sender: string,
    chainId: string,
    accountState: { sequence: number },
  ): Promise<Tx> {
    const { sequence } = accountState;
    const tx = createSignBytes(msgs, sender, chainId, sequence);
    const signature = await this.signBytes(tx);
    return {
      sender,
      msgs,
      credential: encodeBase64(signature),
    };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
