import { encodeBase64 } from "./encoding.js";

import type { StandardCredential } from "./types/credential.js";
import type { KeyHash } from "./types/key.js";
import type { SigningSessionInfo } from "./types/session.js";
import type { Signer } from "./types/signer.js";

export async function createSessionSignature(parameters: {
  signer: Signer;
  pubKey: Uint8Array;
  expireAt: number;
}): Promise<{
  keyHash: KeyHash;
  authorization: StandardCredential;
  sessionInfo: SigningSessionInfo;
}> {
  const { expireAt, signer, pubKey } = parameters;

  const sessionInfo: SigningSessionInfo = {
    sessionKey: encodeBase64(pubKey),
    expireAt: expireAt.toString(),
  };

  const { credential } = await signer.signArbitrary({
    primaryType: "Message" as const,
    message: sessionInfo,
    types: {
      Message: [
        { name: "session_key", type: "string" },
        { name: "expire_at", type: "string" },
      ],
    },
  });

  if ("standard" in credential) {
    return {
      keyHash: credential.standard.keyHash,
      authorization: credential.standard,
      sessionInfo,
    };
  }
  throw new Error("unsupported credential type");
}
