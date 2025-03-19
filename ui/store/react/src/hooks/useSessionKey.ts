import { createSessionSigner, createSignerClient } from "@left-curve/dango";
import { Secp256k1 } from "@left-curve/dango/crypto";
import { createStorage } from "@left-curve/store";

import { useQuery } from "@tanstack/react-query";
import { useAccount } from "./useAccount.js";
import { useConfig } from "./useConfig.js";
import { useConnectorClient } from "./useConnectorClient.js";
import useStorage from "./useStorage.js";

import type { SigningSession } from "@left-curve/dango/types";

export type UseSessionKeyParameters = {
  session?: SigningSession;
};

export type UseSessionKeyReturnType = {
  client: ReturnType<typeof createSignerClient> | undefined;
  session: SigningSession | null;
  deleteSessionkey: () => void;
  createSessionKey: (parameters: { expireAt: number }) => Promise<void>;
};

export function useSessionKey(parameters: UseSessionKeyParameters = {}): UseSessionKeyReturnType {
  const config = useConfig();
  const { username } = useAccount();
  const { data: connectorClient } = useConnectorClient();
  const [session, setSession] = useStorage<SigningSession | null>("session_key", {
    initialValue: parameters.session,
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

    const { authorization, keyHash, sessionInfo } = await connectorClient.createSession({
      expireAt,
      pubKey: publicKey,
    });

    setSession({
      keyHash,
      sessionInfo,
      privateKey: keyPair.privateKey,
      publicKey,
      authorization,
    });
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
