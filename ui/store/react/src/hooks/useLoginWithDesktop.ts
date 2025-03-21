import { encodeBase64, encodeUtf8, serializeJson } from "@left-curve/dango/encoding";

import { useChainId } from "./useChainId.js";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors.js";
import { useDataChannel } from "./useDataChannel.js";

import { Secp256k1 } from "@left-curve/dango/crypto";
import { Actions } from "@left-curve/dango/utils";

import type { SessionResponse } from "@left-curve/dango/types";
import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";

export type UseLoginWithDesktopParameters = {
  url: string;
  username: string;
  expiresAt?: number;
  connectors?: UseConnectorsReturnType;
  mutation?: UseMutationParameters<void, Error, { socketId: string }>;
};

export type UseLoginWithDesktopReturnType = UseMutationReturnType<
  void,
  Error,
  { socketId: string }
>;

export function useLoginWithDesktop(parameters: UseLoginWithDesktopParameters) {
  const {
    url,
    username,
    expiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000),
    mutation,
  } = parameters;
  const connectors = parameters?.connectors ?? useConnectors();
  const chainId = useChainId();

  const { data: dataChannel } = useDataChannel({ url });

  return useMutation({
    mutationFn: async ({ socketId }) => {
      if (!dataChannel) throw new Error("error: missing dataChannel");
      const connector = connectors.find((connector) => connector.id === "session");
      if (!connector) throw new Error("error: missing connector");

      await dataChannel.createPeerConnection(socketId);

      const keyPair = Secp256k1.makeKeyPair();
      const publicKey = keyPair.getPublicKey();

      const response = await dataChannel.sendAsyncMessage<SessionResponse>({
        type: Actions.GenerateSession,
        message: {
          expireAt: +expiresAt,
          publicKey: encodeBase64(publicKey),
        },
      });

      const { authorization, keyHash, sessionInfo } = response;

      await connector.connect({
        username,
        chainId,
        challenge: encodeBase64(
          encodeUtf8(
            serializeJson({
              authorization,
              keyHash,
              sessionInfo,
              publicKey,
              privateKey: keyPair.privateKey,
            }),
          ),
        ),
      });
    },
    ...mutation,
  });
}
