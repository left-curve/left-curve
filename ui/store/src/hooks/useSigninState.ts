import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useSessionKey } from "./useSessionKey.js";
import { useConnectors } from "./useConnectors.js";
import { usePublicClient } from "./usePublicClient.js";
import { useChainId } from "./useChainId.js";

import type { SigningSession, User } from "@left-curve/dango/types";

type ScreenState = "options" | "usernames" | "email" | "wallets";
export type UseSigninStateParameters = {
  session: boolean;
  expiration: number;
  login?: {
    error?: (e: unknown) => void;
    success?: () => void;
  };
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
    users: User[];
    keyHash?: string;
    signingSession?: SigningSession;
    connectorId?: string;
  }>({ users: [] });

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
        const users = await publicClient.forgotUsername({
          keyHash: signingSession.keyHash,
        });

        setAuthData({ users, connectorId, signingSession });
      } else {
        const keyHash = await connector.getKeyHash();
        const users = await publicClient.forgotUsername({ keyHash });
        setAuthData({ users, connectorId, keyHash });
      }
      setScreen("usernames");
    },
  });

  const login = useMutation({
    onError: parameters.login?.error,
    onSuccess: parameters.login?.success,
    mutationFn: async (userIndex: number) => {
      const { connectorId, keyHash, signingSession } = authData;
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      if (!signingSession) {
        await connector.connect({
          userIndex,
          chainId,
          ...(keyHash
            ? { keyHash }
            : { challenge: "Please sign this message to confirm your identity." }),
        });
        return;
      }

      setSession(signingSession);

      await connector.connect({
        userIndex,
        chainId,
        keyHash: signingSession.keyHash,
      });
    },
  });

  return {
    screen,
    setScreen,
    email,
    setEmail,
    users: authData.users,
    connect,
    login,
  };
}
