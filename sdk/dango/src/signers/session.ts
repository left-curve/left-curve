import { Secp256k1, sha256 } from "@left-curve/sdk/crypto";
import { encodeBase64, serialize } from "@left-curve/sdk/encoding";

import type { JsonValue } from "@left-curve/sdk/types";

import type { SessionCredential, SignDoc, Signer, SigningSession } from "../types/index.js";
import type { ArbitraryDoc } from "../types/signature.js";

export const createSessionSigner = (session: SigningSession): Signer => {
  const { sessionInfo, authorization, privateKey, keyHash } = session;
  const signer = new Secp256k1(privateKey);

  async function getKeyHash() {
    return keyHash;
  }

  async function signTx(signDoc: SignDoc) {
    const { message, domain } = signDoc;
    const sender = domain.verifyingContract;
    const { messages, metadata, gas_limit: gasLimit } = message;
    const tx = sha256(
      serialize({
        sender,
        gasLimit,
        messages,
        data: {
          username: metadata.username,
          chainId: metadata.chainId,
          nonce: metadata.nonce,
          expiry: metadata.expiry,
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
      signed: signDoc,
    };
  }

  async function signArbitrary(payload: ArbitraryDoc) {
    const { message } = payload;
    const bytes = sha256(serialize(message));
    const signature = signer.createSignature(bytes);

    const session: SessionCredential = {
      sessionInfo,
      authorization,
      sessionSignature: encodeBase64(signature),
    };

    return {
      credential: { session },
      signed: message,
    };
  }

  return {
    getKeyHash,
    signTx,
    signArbitrary,
  };
};
