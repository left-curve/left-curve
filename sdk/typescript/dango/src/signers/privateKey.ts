import { type KeyPair, Secp256k1, sha256 } from "@left-curve/crypto";
import { encodeBase64, serialize } from "@left-curve/encoding";
import { createKeyHash } from "#account/key.js";

import type { ArbitraryDoc, KeyHash, SignDoc, Signer } from "@left-curve/types";

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
    const bytes = sha256(serialize(toSignDocPayload(signDoc)));
    const signature = await this.signBytes(bytes);
    const keyHash = await this.getKeyHash();

    const credential = {
      standard: {
        signature: { secp256k1: encodeBase64(signature) },
        keyHash,
      },
    };

    return { credential, signed: signDoc };
  }

  async signArbitrary(payload: ArbitraryDoc) {
    const bytes = sha256(serialize(toArbitraryPayload(payload)));
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

function toSignDocPayload(signDoc: SignDoc) {
  return {
    sender: signDoc.sender,
    gasLimit: signDoc.gasLimit,
    messages: signDoc.messages,
    data: signDoc.data,
  };
}

function toArbitraryPayload(payload: ArbitraryDoc) {
  if (payload.kind === "session") {
    return {
      chainId: payload.chainId,
      sessionKey: payload.sessionKey,
      expireAt: payload.expireAt,
    };
  }
  return {
    chainId: payload.chainId,
    key: payload.key,
    keyHash: payload.keyHash,
    seed: payload.seed,
    ...(payload.referrer !== undefined ? { referrer: payload.referrer } : {}),
  };
}
