import { Secp256k1, ripemd160, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";

import type { AbstractSigner } from "@leftcurve/types";
import type { Credential, Message } from "@leftcurve/types";

export type WalletEvm = {
  getPublicKey(): Promise<Uint8Array>;
  signMessage(message: string | Uint8Array): Promise<Uint8Array>;
  signBytes(bytes: Uint8Array): Promise<Uint8Array>;
};

export class WalletEvmSigner implements AbstractSigner {
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

  async signTx(msgs: Message[], chainId: string, sequence: number): Promise<Credential> {
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));

    const signature = await this.#wallet.signBytes(tx);
    return { secp256k1: encodeBase64(signature) };
  }
}
