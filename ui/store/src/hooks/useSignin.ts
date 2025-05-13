import { Secp256k1 } from "@left-curve/dango/crypto";
import { encodeBase64 } from "@left-curve/dango/encoding";
import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";
import { useChainId } from "./useChainId.js";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors.js";
import { useSessionKey } from "./useSessionKey.js";

import type { KeyHash, Username } from "@left-curve/dango/types";

export type UseSigninParameters = {
  connectors?: UseConnectorsReturnType;
  sessionKey?:
    | {
        expireAt: number;
      }
    | false;
  mutation?: UseMutationParameters<
    Username,
    Error,
    { connectorId: string; username: Username; keyHash?: KeyHash }
  >;
};

export type UseSigninReturnType = UseMutationReturnType<
  Username,
  Error,
  { connectorId: string; usename: Username; keyHash?: KeyHash }
>;

export function useSignin(parameters: UseSigninParameters) {
  const { mutation, sessionKey = false } = parameters;
  const connectors = parameters?.connectors ?? useConnectors();
  const chainId = useChainId();
  const { setSession } = useSessionKey();

  return useMutation({
    mutationFn: async ({ connectorId, username, keyHash }) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      if (!sessionKey) {
        await connector.connect({
          username,
          chainId,
          ...(keyHash
            ? { keyHash }
            : { challenge: "Please sign this message to confirm your identity." }),
        });
        return username;
      }

      const keyPair = Secp256k1.makeKeyPair();
      const publicKey = keyPair.getPublicKey();

      const sessionInfo = {
        sessionKey: encodeBase64(publicKey),
        expireAt: sessionKey.expireAt.toString(),
      };

      const { credential } = await connector.signArbitrary({
        primaryType: "Message" as const,
        message: sessionInfo,
        types: {
          Message: [
            { name: "session_key", type: "string" },
            { name: "expire_at", type: "string" },
          ],
        },
      });

      if ("session" in credential) throw new Error("unsupported credential type");

      await connector.connect({
        username,
        chainId,
        keyHash: credential.standard.keyHash,
      });

      setSession({
        keyHash: credential.standard.keyHash,
        sessionInfo,
        privateKey: keyPair.privateKey,
        publicKey,
        authorization: credential.standard,
      });

      return username;
    },
    ...mutation,
  });
}
