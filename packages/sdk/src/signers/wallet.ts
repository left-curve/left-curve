import { Secp256k1, ripemd160, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";

import type { Signer } from "@leftcurve/types";
import type { Message } from "@leftcurve/types";

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

  async getKeyHash(): Promise<string> {
    const publicKey = await this.#wallet.getPublicKey();
    const keyHash = ripemd160(publicKey);
    const signature = await this.#wallet.signMessage(keyHash);
    const verified = Secp256k1.verifySignature(keyHash, signature, publicKey);
    if (!verified) {
      throw new Error("invalid signature");
    }
    return encodeHex(keyHash);
  }

  async signTx(msgs: Message[], chainId: string, sequence: number) {
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));
    const signature = await this.#wallet.signBytes(tx);

    const credential = { secp256k1: encodeBase64(signature) };
    const keyHash = await this.getKeyHash();

    return { credential, keyHash };
  }
}
