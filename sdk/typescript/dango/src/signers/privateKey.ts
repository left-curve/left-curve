import { Secp256k1, sha256 } from "@left-curve/sdk/crypto";
import { encodeBase64, serialize } from "@left-curve/sdk/encoding";
import { createKeyHash } from "../account/key.js";

import type { KeyPair } from "@left-curve/sdk/crypto";
import type { ArbitraryTypedData, KeyHash, SignDoc, Signer } from "../types/index.js";

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
    return createKeyHash(this.#keyPair.getPublicKey());
  }

  async signTx(signDoc: SignDoc) {
    const { message } = signDoc;
    const tx = sha256(serialize(message));

    const signature = await this.signBytes(tx);

    const keyHash = await this.getKeyHash();

    const credential = {
      standard: {
        signature: { secp256k1: encodeBase64(signature) },
        keyHash,
      },
    };

    return { credential, signed: signDoc };
  }

  async signArbitrary(payload: ArbitraryTypedData) {
    const { message } = payload;
    const bytes = sha256(serialize(message));
    const signedBytes = await this.signBytes(bytes);

    const signature = { secp256k1: encodeBase64(signedBytes) };
    const keyHash = await this.getKeyHash();

    return {
      credential: { standard: { keyHash, signature } },
      signed: payload,
    };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
