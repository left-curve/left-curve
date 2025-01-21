import { Secp256k1 } from "@left-curve/crypto";
import { encodeBase64 } from "@left-curve/encoding";
import { createSessionSigner, createSignerClient } from "@left-curve/sdk";
import type { SigningSession, SigningSessionInfo } from "@left-curve/types";
import { useQuery } from "@tanstack/react-query";
import {
  createStorage,
  useAccount,
  useConfig,
  useConnectorClient,
  useStorage,
} from "../../../../../sdk/packages/dango/src/store/react";

export function useSessionKey() {
  const config = useConfig();
  const { username } = useAccount();
  const { data: connectorClient } = useConnectorClient();
  const [session, setSession] = useStorage<SigningSession | null>("session_key", {
    storage: createStorage({ storage: sessionStorage }),
    version: 1,
  });

  const { data: client } = useQuery({
    queryKey: ["session_key", session],
    queryFn: async () => {
      if (!session || !username) return undefined;
      return createSignerClient({
        username,
        signer: createSessionSigner(session),
        transport: config._internal.transports[config.state.chainId],
      });
    },
  });

  async function createSessionKey(parameters: {
    expireAt: number;
  }) {
    if (!connectorClient) return;
    const { expireAt } = parameters;
    const keyPair = Secp256k1.makeKeyPair();
    const publicKey = keyPair.getPublicKey();

    const sessionInfo: SigningSessionInfo = {
      sessionKey: encodeBase64(publicKey),
      expireAt: expireAt.toString(),
    };

    const typedData = {
      primaryType: "Message",
      message: sessionInfo,
      types: {
        Message: [
          { name: "session_key", type: "string" },
          { name: "expire_at", type: "string" },
        ],
      },
    };

    const payload = connectorClient.type === "eip1193" ? typedData : sessionInfo;
    const { credential } = await connectorClient.signer.signArbitrary(payload);

    if ("standard" in credential) {
      const session: SigningSession = {
        keyHash: credential.standard.keyHash,
        sessionInfo,
        privateKey: keyPair.privateKey,
        publicKey: keyPair.getPublicKey(),
        authorization: credential.standard,
      };
      setSession(session);
    } else throw new Error("unsupported credential type");
  }

  function deleteSessionkey() {
    setSession(null);
  }

  return {
    client,
    session,
    deleteSessionkey,
    createSessionKey,
  };
}
