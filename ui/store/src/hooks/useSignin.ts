import { Secp256k1 } from "@left-curve/dango/crypto";
import { encodeBase64 } from "@left-curve/dango/encoding";
import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";
import { useChainId } from "./useChainId.js";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors.js";
import { useSessionKey } from "./useSessionKey.js";

export type UseSigninParameters = {
  username: string;
  connectors?: UseConnectorsReturnType;
  sessionKey?: boolean;
  mutation?: UseMutationParameters<void, Error, { connectorId: string }>;
};

export type UseSigninReturnType = UseMutationReturnType<void, Error, { connectorId: string }>;

export function useSignin(parameters: UseSigninParameters) {
  const { username, mutation, sessionKey = false } = parameters;
  const connectors = parameters?.connectors ?? useConnectors();
  const chainId = useChainId();
  const { setSession } = useSessionKey();

  return useMutation({
    mutationFn: async ({ connectorId }) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      if (!sessionKey) {
        return await connector.connect({
          username,
          chainId,
          challenge: "Please sign this message to confirm your identity.",
        });
      }

      const keyPair = Secp256k1.makeKeyPair();
      const publicKey = keyPair.getPublicKey();

      const expireAt = Date.now() + 1000 * 60 * 60 * 24;

      const sessionInfo = {
        sessionKey: encodeBase64(publicKey),
        expireAt: expireAt.toString(),
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
    },
    ...mutation,
  });
}
