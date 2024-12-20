import { Secp256k1 } from "@left-curve/crypto";
import { encodeBase64 } from "@left-curve/encoding";
import {
  createStorage,
  useAccount,
  useConfig,
  useConnectorClient,
  useStorage,
} from "@left-curve/react";
import { createSessionSigner, createSignerClient } from "@left-curve/sdk";
import type { SigningSession, SigningSessionInfo } from "@left-curve/types";
import { useQuery } from "@tanstack/react-query";

export function useSessionKey() {
  const config = useConfig();
  const { username } = useAccount();
  const { data: connectorClient } = useConnectorClient();
  const [session, setSession] = useStorage<SigningSession>("session_key", {
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
    const { signature, keyHash } = await connectorClient.signer.signArbitrary(payload);

    const session: SigningSession = {
      keyHash,
      sessionInfo,
      privateKey: keyPair.privateKey,
      publicKey: keyPair.getPublicKey(),
      authorization: {
        keyHash,
        signature,
      },
    };

    setSession(session);
  }

  return {
    client,
    createSessionKey,
    session,
  };
}
