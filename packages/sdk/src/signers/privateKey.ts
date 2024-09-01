import { ripemd160, sha256 } from "@leftcurve/crypto";
import { Secp256k1 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";

import type { KeyPair } from "@leftcurve/crypto";
import type { Credential, Signer } from "@leftcurve/types";
import type { Message } from "@leftcurve/types";

export class PrivateKeySigner implements Signer {
  #keyPair: KeyPair;

  static fromMnemonic(mnemonic: string): PrivateKeySigner {
    return new PrivateKeySigner(Secp256k1.fromMnemonic(mnemonic));
  }

  static fromPrivateKey(privateKey: Uint8Array): PrivateKeySigner {
    return new PrivateKeySigner(new Secp256k1(privateKey));
  }

  static fromRandomKey(): PrivateKeySigner {
    return new PrivateKeySigner(Secp256k1.makeKeyPair());
  }

  constructor(keyPair: KeyPair) {
    this.#keyPair = keyPair;
  }

  async getKeyId(): Promise<string> {
    return encodeHex(ripemd160(this.#keyPair.publicKey)).toUpperCase();
  }

  async signTx(msgs: Message[], chainId: string, sequence: number) {
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));
    const signature = await this.signBytes(tx);

    const credential = { secp256k1: encodeBase64(signature) };
    const data = { keyHash: await this.getKeyId(), sequence };

    return { credential, data };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
