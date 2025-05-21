import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";
import { useChainId } from "./useChainId.js";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors.js";
import { useSessionKey } from "./useSessionKey.js";

import type { KeyHash, SigningSession, Username } from "@left-curve/dango/types";

export type UseSigninParameters = {
  connectors?: UseConnectorsReturnType;
  session?: false | SigningSession | { expireAt: number };
  mutation?: UseMutationParameters<
    Username,
    Error,
    { connectorId: string; username: Username; keyHash?: KeyHash }
  >;
};

export type UseSigninReturnType = UseMutationReturnType<
  Username,
  Error,
  { connectorId: string; username: Username; keyHash?: KeyHash }
>;

export function useSignin(parameters: UseSigninParameters) {
  const { mutation, session } = parameters;
  const connectors = parameters?.connectors ?? useConnectors();
  const chainId = useChainId();
  const { createSessionKey, setSession } = useSessionKey();

  return useMutation({
    mutationFn: async ({ connectorId, username, keyHash }) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      if (!session) {
        await connector.connect({
          username,
          chainId,
          ...(keyHash
            ? { keyHash }
            : { challenge: "Please sign this message to confirm your identity." }),
        });
        return username;
      }

      const signingSession = await (async () => {
        if ("authorization" in session) return session;
        return await createSessionKey(
          { connector, expireAt: session.expireAt },
          { setSession: false },
        );
      })();

      setSession(signingSession);

      await connector.connect({
        username,
        chainId,
        keyHash: signingSession.keyHash,
      });

      return username;
    },
    ...mutation,
  });
}
