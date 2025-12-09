import { useRef, useState } from "react";
import { useConnectors } from "./useConnectors.js";
import { usePublicClient } from "./usePublicClient.js";
import { useConfig } from "./useConfig.js";
import { createKeyHash } from "@left-curve/dango";
import { registerUser } from "@left-curve/dango/actions";

import { useMutation } from "@tanstack/react-query";
import { useChainId } from "./useChainId.js";
import type { Address, Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "../types/eip1193.js";
import { useSessionKey } from "./useSessionKey.js";
import type { Connector } from "../types/connector.js";

type ScreenState = "options" | "email" | "wallets" | "login" | "deposit";

export type UseSignupStateParameters = {
  expiration: number;
  login?: {
    onError?: (error: unknown) => void;
    onSuccess?: () => void;
  };
  register?: {
    onError?: (error: unknown) => void;
    onSuccess?: () => void;
  };
};

export function useSignupState(parameters: UseSignupStateParameters) {
  const { expiration } = parameters;
  const [screen, setScreen] = useState<ScreenState>("options");
  const [email, setEmail] = useState<string>("");
  const chainId = useChainId();
  const connectors = useConnectors();
  const client = usePublicClient();
  const config = useConfig();
  const connectorRef = useRef<Connector | null>(null);
  const { createSessionKey, setSession } = useSessionKey();

  const register = useMutation({
    onError: parameters.register?.onError,
    onSuccess: parameters.register?.onSuccess,
    mutationFn: async (connectorId: string) => {
      const connector = connectors.find((c) => c.id === connectorId);
      if (!connector) throw new Error("error: missing connector");
      connectorRef.current = connector;

      const challenge = "Please sign this message to confirm your identity.";

      const { key, keyHash } = await (async () => {
        if (connectorId === "passkey") {
          return connector.createNewKey!(challenge);
        }
        const provider = await (
          connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
        ).getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const addressLowerCase = controllerAddress.toLowerCase() as Address;
        return {
          key: { ethereum: addressLowerCase } as Key,
          keyHash: createKeyHash(addressLowerCase),
        };
      })();

      const { credential } = await connector.signArbitrary({
        primaryType: "Message" as const,
        message: { chain_id: config.chain.id },
        types: { Message: [{ name: "chain_id", type: "string" }] },
      });

      if (!("standard" in credential)) throw new Error("Signed with wrong credential");

      await registerUser(client, {
        key,
        keyHash,
        seed: Math.floor(Math.random() * 0x100000000),
        signature: credential.standard.signature,
      });
      setScreen("login");
    },
  });

  const login = useMutation({
    onError: parameters.login?.onError,
    onSuccess: parameters.login?.onSuccess,
    mutationFn: async (parameters: { useSessionKey: boolean }) => {
      const { useSessionKey } = parameters;

      const connector = connectorRef.current!;

      const { account, keyHash, signingSession } = await (async () => {
        if (useSessionKey) {
          const signingSession = await createSessionKey(
            { connector, expireAt: Date.now() + expiration },
            { setSession: false },
          );
          const usersIndexAndName = await client.forgotUsername({
            keyHash: signingSession.keyHash,
          });

          return { account: usersIndexAndName[usersIndexAndName.length - 1], signingSession };
        } else {
          const keyHash = await connector.getKeyHash();
          const usersIndexAndName = await client.forgotUsername({ keyHash });
          return { account: usersIndexAndName[usersIndexAndName.length - 1], keyHash };
        }
      })();

      if (!signingSession) {
        return await connector.connect({
          userIndexAndName: account,
          chainId,
          ...(keyHash
            ? { keyHash }
            : { challenge: "Please sign this message to confirm your identity." }),
        });
      }

      setSession(signingSession);

      await connector.connect({
        userIndexAndName: account,
        chainId,
        keyHash: signingSession.keyHash,
      });
      setScreen("deposit");
    },
  });

  return {
    login,
    register,
    screen,
    setScreen,
    email,
    setEmail,
  };
}
