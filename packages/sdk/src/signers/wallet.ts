import { ripemd160, sha256, verifySignature } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";

import type { KeyHash, SignDoc, Signer } from "@leftcurve/types";

export type WalletEvm = {
  getPublicKey(): Promise<Uint8Array>;
  signMessage(message: string | Uint8Array): Promise<Uint8Array>;
  signBytes(bytes: Uint8Array): Promise<Uint8Array>;
};

export class WalletEvmSigner implements Signer {
  #wallet: WalletEvm;

  constructor(wallet: WalletEvm) {
    this.#wallet = wallet;
  }

  async getKeyHash(): Promise<KeyHash> {
    const publicKey = await this.#wallet.getPublicKey();
    const keyHash = ripemd160(publicKey);
    const signature = await this.#wallet.signMessage(keyHash);
    const verified = verifySignature(keyHash, signature, publicKey);
    if (!verified) {
      throw new Error("invalid signature");
    }
    return encodeHex(keyHash);
  }

  async signTx(signDoc: SignDoc) {
    const { msgs, chainId, sequence } = signDoc;
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));
    const signature = await this.#wallet.signBytes(tx);

    const credential = { secp256k1: encodeBase64(signature) };
    const keyHash = await this.getKeyHash();

    return { credential, keyHash };
  }
}
