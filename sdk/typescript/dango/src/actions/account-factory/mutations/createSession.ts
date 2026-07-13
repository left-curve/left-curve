import { encodeBase64 } from "@left-curve/encoding";

import type {
  Client,
  KeyHash,
  Signer,
  SigningSessionInfo,
  StandardCredential,
} from "@left-curve/types";

export type CreateSessionParameters = {
  pubKey: Uint8Array;
  expireAt: number;
};

export type CreateSessionReturnType = Promise<{
  keyHash: KeyHash;
  authorization: StandardCredential;
  sessionInfo: SigningSessionInfo;
}>;

export async function createSession(
  client: Client<Signer>,
  parameters: CreateSessionParameters,
): CreateSessionReturnType {
  const { expireAt, pubKey } = parameters;

  if (!client.chain) throw new Error("chain is required for session creation");

  const sessionInfo: SigningSessionInfo = {
    chainId: client.chain.id,
    sessionKey: encodeBase64(pubKey),
    expireAt: Math.floor(expireAt / 1000).toString(),
  };

  const { credential } = await client.signer.signArbitrary({
    kind: "session",
    chainId: sessionInfo.chainId,
    sessionKey: sessionInfo.sessionKey,
    expireAt: sessionInfo.expireAt,
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
