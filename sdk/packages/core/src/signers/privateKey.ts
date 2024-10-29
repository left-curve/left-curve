import { Ed25519, sha256 } from "@leftcurve/crypto";
import { Secp256k1 } from "@leftcurve/crypto";
import { encodeBase64, serialize } from "@leftcurve/encoding";
import { createKeyHash } from "../accounts/index.js";

import type { KeyPair } from "@leftcurve/crypto";
import {
  KeyAlgo,
  type KeyAlgoType,
  type KeyHash,
  type SignDoc,
  type Signer,
} from "@leftcurve/types";

export class PrivateKeySigner implements Signer {
  #keyPair: KeyPair;
  #keyAlgo: KeyAlgoType;

  static fromMnemonic(
    mnemonic: string,
    keyAlgo: KeyAlgoType = KeyAlgo.Secp256k1,
  ): PrivateKeySigner {
    const key = (() => {
      if (keyAlgo === KeyAlgo.Ed25519) return Ed25519.fromMnemonic(mnemonic);
      if (keyAlgo === KeyAlgo.Secp256k1) return Secp256k1.fromMnemonic(mnemonic);
      throw new Error(`unsupported key algorithm: ${keyAlgo}`);
    })();

    return new PrivateKeySigner(key, keyAlgo);
  }

  static fromPrivateKey(
    privateKey: Uint8Array,
    keyAlgo: KeyAlgoType = KeyAlgo.Secp256k1,
  ): PrivateKeySigner {
    const key = (() => {
      if (keyAlgo === KeyAlgo.Ed25519) return new Ed25519(privateKey);
      if (keyAlgo === KeyAlgo.Secp256k1) return new Secp256k1(privateKey);
      throw new Error(`unsupported key algorithm: ${keyAlgo}`);
    })();
    return new PrivateKeySigner(key, keyAlgo);
  }

  static fromRandomKey(keyAlgo: KeyAlgoType = KeyAlgo.Secp256k1): PrivateKeySigner {
    const key = (() => {
      if (keyAlgo === KeyAlgo.Ed25519) return Ed25519.makeKeyPair();
      if (keyAlgo === KeyAlgo.Secp256k1) return Secp256k1.makeKeyPair();
      throw new Error(`unsupported key algorithm: ${keyAlgo}`);
    })();
    return new PrivateKeySigner(key, keyAlgo);
  }

  constructor(keyPair: KeyPair, keyAlgo: KeyAlgoType) {
    this.#keyAlgo = keyAlgo;
    this.#keyPair = keyPair;
  }

  async getKeyHash(): Promise<KeyHash> {
    return createKeyHash({
      pubKey: this.#keyPair.getPublicKey(),
      keyAlgo: this.#keyAlgo,
    });
  }

  async signTx(signDoc: SignDoc) {
    const { messages, chainId, sequence, sender } = signDoc;
    const tx = sha256(serialize({ sender, messages, chainId, sequence }));

    const signature = await this.signBytes(tx);

    const credential = { secp256k1: encodeBase64(signature) };
    const keyHash = await this.getKeyHash();

    return { credential, keyHash };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
