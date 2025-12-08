import { useState } from "react";
import { useConnectors } from "./useConnectors.js";
import { usePublicClient } from "./usePublicClient.js";
import { useConfig } from "./useConfig.js";
import { useSubmitTx } from "./useSubmitTx.js";
import { createKeyHash } from "@left-curve/dango";
import { registerUser } from "@left-curve/dango/actions";

import type { Address, Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "../types/eip1193.js";

type ScreenState = "options" | "email" | "wallets";

export type UseSignupStateParameters = {
  toast?: {
    error?: (error: unknown) => void;
    success?: () => void;
  };
};

export function useSignupState(params: UseSignupStateParameters = {}) {
  const { toast } = params;
  const [screen, setScreen] = useState<ScreenState>("options");
  const [email, setEmail] = useState<string>("");
  const connectors = useConnectors();
  const client = usePublicClient();
  const config = useConfig();

  const submission = useSubmitTx({
    toast: {
      ...toast,
    },
    mutation: {
      mutationFn: async (connectorId: string) => {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");

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
      },
    },
  });

  return {
    submission,
    screen,
    setScreen,
    email,
    setEmail,
  };
}
