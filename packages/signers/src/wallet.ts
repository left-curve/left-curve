import { Secp256k1, ripemd160 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex } from "@leftcurve/encoding";
import { createSignBytes } from "@leftcurve/types";

import type { AbstractSigner } from "@leftcurve/types";
import type { Message, Tx } from "@leftcurve/types";

export type WalletEvm = {
  getPublicKey(): Promise<Uint8Array>;
  signMessage(message: string | Uint8Array): Promise<Uint8Array>;
  signBytes(bytes: Uint8Array): Promise<Uint8Array>;
};

export class WalletEvmSigner implements AbstractSigner<{ sequence: number }> {
  #wallet: WalletEvm;

  constructor(wallet: WalletEvm) {
    this.#wallet = wallet;
  }

  async getKeyId(): Promise<string> {
    const publicKey = await this.#wallet.getPublicKey();
    const keyId = ripemd160(publicKey);
    const signature = await this.#wallet.signMessage(keyId);
    const verified = Secp256k1.verifySignature(keyId, signature, publicKey);
    if (!verified) {
      throw new Error("invalid signature");
    }
    return encodeHex(keyId);
  }

  async signTx(
    msgs: Message[],
    sender: string,
    chainId: string,
    accountState: { sequence: number },
  ): Promise<Tx> {
    const { sequence } = accountState;
    const tx = createSignBytes(msgs, sender, chainId, sequence);

    const signature = await this.#wallet.signBytes(tx);

    return {
      sender,
      msgs,
      credential: encodeBase64(signature),
    };
  }
}
