import { Secp256k1, sha256 } from "@left-curve/crypto";
import { encodeBase64, serialize } from "@left-curve/encoding";
import type { SessionCredential, SignDoc, Signer, SigningSession } from "@left-curve/types";
import { createKeyHash } from "../accounts/key.js";

export const createSessionSigner = (session: SigningSession): Signer => {
  const { sessionInfo, sessionInfoSignature, privateKey, publicKey } = session;
  const signer = new Secp256k1(privateKey);

  async function getKeyHash() {
    return createKeyHash({
      pubKey: publicKey,
      keyAlgo: "secp256k1",
    });
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
    const keyHash = await getKeyHash();

    return { credential, keyHash };
  }

  return {
    getKeyHash,
    signTx,
  };
};
