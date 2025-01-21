import { Ed25519, sha256 } from "@left-curve/crypto";
import { Secp256k1 } from "@left-curve/crypto";
import { encodeBase64, serialize } from "@left-curve/encoding";
import { createKeyHash } from "../account/key.js";
import { KeyAlgo } from "../types/key.js";

import type { KeyPair } from "@left-curve/crypto";
import type { JsonValue } from "@left-curve/types";
import type { KeyAlgoType, KeyHash } from "../types/key.js";
import type { SignDoc } from "../types/signature.js";
import type { Signer } from "../types/signer.js";

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
    const { messages, sender, data, gasLimit } = signDoc;
    const tx = sha256(
      serialize({
        sender,
        gasLimit,
        messages,
        data: {
          username: data.username,
          chainId: data.chainId,
          nonce: data.nonce,
          expiry: data.expiry,
        },
      }),
    );

    const signature = await this.signBytes(tx);

    const keyHash = await this.getKeyHash();

    const credential = {
      standard: {
        signature: { secp256k1: encodeBase64(signature) },
        keyHash,
      },
    };

    return { credential, signDoc };
  }

  async signArbitrary(payload: JsonValue) {
    const bytes = sha256(serialize(payload));
    const signedBytes = await this.signBytes(bytes);

    const signature = { secp256k1: encodeBase64(signedBytes) };
    const keyHash = await this.getKeyHash();

    return {
      credential: { standard: { keyHash, signature } },
      payload,
    };
  }

  async signBytes(bytes: Uint8Array): Promise<Uint8Array> {
    return this.#keyPair.createSignature(bytes);
  }
}
