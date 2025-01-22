import { Secp256k1, sha256 } from "@left-curve/crypto";
import { encodeBase64, serialize } from "@left-curve/encoding";

import type { JsonValue } from "@left-curve/types";

import type { SessionCredential, SignDoc, Signer, SigningSession } from "../types/index.js";

export const createSessionSigner = (session: SigningSession): Signer => {
  const { sessionInfo, authorization, privateKey, keyHash } = session;
  const signer = new Secp256k1(privateKey);

  async function getKeyHash() {
    return keyHash;
  }

  async function signTx(signDoc: SignDoc) {
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
    const signature = signer.createSignature(tx);

    const session: SessionCredential = {
      sessionInfo,
      authorization,
      sessionSignature: encodeBase64(signature),
    };

    return {
      credential: { session },
      signDoc,
    };
  }

  async function signArbitrary(payload: JsonValue) {
    const bytes = sha256(serialize(payload));
    const signature = signer.createSignature(bytes);

    const session: SessionCredential = {
      sessionInfo,
      authorization,
      sessionSignature: encodeBase64(signature),
    };

    return {
      credential: { session },
      payload,
    };
  }

  return {
    getKeyHash,
    signTx,
    signArbitrary,
  };
};
