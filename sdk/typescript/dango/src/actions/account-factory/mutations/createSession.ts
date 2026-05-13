import { encodeBase64 } from "@left-curve/sdk/encoding";

import type { Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../../types/clients.js";
import type { StandardCredential } from "../../../types/credential.js";
import type { KeyHash } from "../../../types/key.js";
import type { SigningSessionInfo } from "../../../types/session.js";
import type { Signer } from "../../../types/signer.js";

export type CreateSessionParameters = {
  pubKey: Uint8Array;
  expireAt: number;
};

export type CreateSessionReturnType = Promise<{
  keyHash: KeyHash;
  authorization: StandardCredential;
  sessionInfo: SigningSessionInfo;
}>;

export async function createSession<transport extends Transport>(
  client: DangoClient<transport, Signer>,
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
    primaryType: "Message" as const,
    message: sessionInfo,
    types: {
      Message: [
        { name: "chain_id", type: "string" },
        { name: "expire_at", type: "string" },
        { name: "session_key", type: "string" },
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
