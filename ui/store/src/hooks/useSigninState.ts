import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useSessionKey } from "./useSessionKey.js";
import { useConnectors } from "./useConnectors.js";
import { usePublicClient } from "./usePublicClient.js";
import { useChainId } from "./useChainId.js";

import type { SigningSession, Username } from "@left-curve/dango/types";

type ScreenState = "options" | "usernames" | "email" | "wallets";
export type UseSigninStateParameters = {
  session: boolean;
  expiration: number;
  connect?: {
    error?: (e: unknown) => void;
    success?: () => void;
  };
};

export function useSigninState(parameters: UseSigninStateParameters) {
  const { session, expiration } = parameters;

  const { createSessionKey, setSession } = useSessionKey();
  const connectors = useConnectors();
  const publicClient = usePublicClient();
  const chainId = useChainId();

  const [email, setEmail] = useState<string>("");
  const [screen, setScreen] = useState<ScreenState>("options");
  const [authData, setAuthData] = useState<{
    usernames: Username[];
    keyHash?: string;
    signingSession?: SigningSession;
    connectorId?: string;
  }>({ usernames: [] });

  const connect = useMutation({
    onError: parameters.connect?.error,
    onSuccess: parameters.connect?.success,
    mutationFn: async (connectorId: string) => {
      const connector = connectors.find((c) => c.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      if (session) {
        const signingSession = await createSessionKey(
          { connector, expireAt: Date.now() + expiration },
          { setSession: false },
        );
        const usernames = await publicClient.forgotUsername({
          keyHash: signingSession.keyHash,
        });

        setAuthData({ usernames, connectorId, signingSession });
      } else {
        const keyHash = await connector.getKeyHash();
        const usernames = await publicClient.forgotUsername({ keyHash });
        setAuthData({ usernames, connectorId, keyHash });
      }
    },
  });

  const login = useMutation({
    mutationFn: async (username: string) => {
      const { connectorId, keyHash, signingSession } = authData;
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      if (!signingSession) {
        await connector.connect({
          username,
          chainId,
          ...(keyHash
            ? { keyHash }
            : { challenge: "Please sign this message to confirm your identity." }),
        });
        return username;
      }

      setSession(signingSession);

      await connector.connect({
        username,
        chainId,
        keyHash: signingSession.keyHash,
      });

      return username;
    },
  });

  return {
    screen,
    setScreen,
    email,
    setEmail,
    usernames: authData.usernames,
    connect,
    login,
  };
}
