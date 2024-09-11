import { sha256 } from "@leftcurve/crypto";
import { Secp256k1 } from "@leftcurve/crypto";
import { encodeBase64, serialize } from "@leftcurve/encoding";
import { createKeyHash } from "../accounts";

import type { KeyPair } from "@leftcurve/crypto";
import type { KeyHash, SignDoc, Signer } from "@leftcurve/types";

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

  async getKeyHash(): Promise<KeyHash> {
    return createKeyHash({ pubKey: this.#keyPair.publicKey });
  }

  async signTx(signDoc: SignDoc) {
    const { typedData, ...txMessage } = signDoc;
    const tx = sha256(serialize(txMessage));
    const signature = await this.signBytes(tx);

    const credential = { secp256k1: encodeBase64(signature) };
    const keyHash = await this.getKeyHash();

    return { credential, keyHash };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
