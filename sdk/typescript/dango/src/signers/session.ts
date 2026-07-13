import { Secp256k1, sha256 } from "@left-curve/crypto";
import { encodeBase64, serialize } from "@left-curve/encoding";

import type {
  ArbitraryDoc,
  SessionCredential,
  SignDoc,
  Signer,
  SigningSession,
} from "@left-curve/types";

export const createSessionSigner = (session: SigningSession): Signer => {
  const { sessionInfo, authorization, privateKey, keyHash } = session;
  const signer = new Secp256k1(privateKey);

  async function getKeyHash() {
    return keyHash;
  }

  async function signTx(signDoc: SignDoc) {
    const bytes = sha256(serialize(toSignDocPayload(signDoc)));
    const signature = signer.createSignature(bytes);

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
    const bytes = sha256(serialize(toArbitraryPayload(payload)));
    const signature = signer.createSignature(bytes);

    const session: SessionCredential = {
      sessionInfo,
      authorization,
      sessionSignature: encodeBase64(signature),
    };

    return {
      credential: { session },
      signed: payload,
    };
  }

  return {
    getKeyHash,
    signTx,
    signArbitrary,
  };
};

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
