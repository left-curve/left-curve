import { encodeBase64, encodeUtf8, serializeJson } from "@left-curve/dango/encoding";

import { useChainId } from "./useChainId.js";
import { useConnectors } from "./useConnectors.js";
import { useSubmitTx } from "./useSubmitTx.js";

import { Secp256k1 } from "@left-curve/dango/crypto";
import { Actions, DataChannel } from "@left-curve/dango/utils";

import type { NestedOmit, Result, SessionResponse } from "@left-curve/dango/types";
import type { UseConnectorsReturnType } from "./useConnectors.js";
import type { UseSubmitTxParameters, UseSubmitTxReturnType } from "./useSubmitTx.js";

export type UseSigninWithDesktopParameters = {
  url: string;
  expiresAt?: number;
  connectors?: UseConnectorsReturnType;
} & NestedOmit<UseSubmitTxParameters<void, Error, { socketId: string }>, "mutation.mutationFn">;

export type UseSigninWithDesktopReturnType = UseSubmitTxReturnType<
  void,
  Error,
  { socketId: string }
>;

export function useSigninWithDesktop(parameters: UseSigninWithDesktopParameters) {
  const { url, expiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000), mutation } = parameters;
  const fallbackConnectors = useConnectors();
  const connectors = parameters?.connectors ?? fallbackConnectors;
  const chainId = useChainId();

  return useSubmitTx({
    mutation: {
      ...mutation,
      mutationFn: async ({ socketId }) => {
        const dataChannel = await DataChannel.create(url);
        const connector = connectors.find((connector) => connector.id === "session");
        if (!connector) throw new Error("error: missing connector");

        await dataChannel.createPeerConnection(socketId);

        const keyPair = Secp256k1.makeKeyPair();
        const publicKey = keyPair.getPublicKey();

        const { error, data } = await dataChannel.sendAsyncMessage<
          Result<SessionResponse & { username: string }>
        >({
          type: Actions.GenerateSession,
          message: {
            expireAt: +expiresAt,
            publicKey: encodeBase64(publicKey),
          },
        });

        if (error) throw error;

        const { authorization, keyHash, sessionInfo, username } = data;

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
    },
  });
}
