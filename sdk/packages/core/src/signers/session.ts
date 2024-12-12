import { Secp256k1, sha256 } from "@left-curve/crypto";
import { encodeBase64, serialize } from "@left-curve/encoding";

import type {
  JsonValue,
  SessionCredential,
  SignDoc,
  Signer,
  SigningSession,
} from "@left-curve/types";

export const createSessionSigner = (session: SigningSession): Signer => {
  const { sessionInfo, sessionInfoSignature, privateKey, keyHash } = session;
  const signer = new Secp256k1(privateKey);

  async function getKeyHash() {
    return keyHash;
  }

  async function signTx(signDoc: SignDoc) {
    const { messages, chainId, sequence, sender } = signDoc;
    const bytes = sha256(serialize({ sender, messages, chainId, sequence }));
    const signature = signer.createSignature(bytes);

    const session: SessionCredential = {
      sessionInfo,
      sessionInfoSignature,
      sessionSignature: encodeBase64(signature),
    };

    const credential = { session };

    return { credential, keyHash };
  }

  async function signArbitrary(payload: JsonValue) {
    const bytes = sha256(serialize(payload));
    const signature = signer.createSignature(bytes);
    const secp256k1Signature = { secp256k1: encodeBase64(signature) };
    return { signature: secp256k1Signature, keyHash };
  }

  return {
    getKeyHash,
    signTx,
    signArbitrary,
  };
};
