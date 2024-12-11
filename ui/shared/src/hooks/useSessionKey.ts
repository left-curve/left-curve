import { Secp256k1 } from "@left-curve/crypto";
import { encodeBase64 } from "@left-curve/encoding";
import {
  createStorage,
  useAccount,
  useConfig,
  useConnectorClient,
  useStorage,
} from "@left-curve/react";
import { createSessionSigner, createSignerClient } from "@left-curve/sdk";
import type { Address, SigningSession, SigningSessionInfo } from "@left-curve/types";
import { useMemo } from "react";

export function useSessionKey() {
  const config = useConfig();
  const { username } = useAccount();
  const { data: connectorClient } = useConnectorClient();
  const [session, setSession] = useStorage<SigningSession>("session_key", {
    storage: createStorage({ storage: sessionStorage }),
    version: 1,
  });

  const client = useMemo(() => {
    if (!session || !username) return undefined;
    return createSignerClient({
      username,
      signer: createSessionSigner(session),
      transport: config._internal.transports[config.state.chainId],
    });
  }, [session]);

  async function createSessionKey(parameters: {
    expireAt: number;
    whitelistedAccounts: Address[];
  }) {
    if (!connectorClient) return;
    const { expireAt, whitelistedAccounts } = parameters;
    const keyPair = Secp256k1.makeKeyPair();
    const publicKey = keyPair.getPublicKey();

    const sessionInfo: SigningSessionInfo = {
      sessionKey: encodeBase64(publicKey),
      whitelistedAccounts,
      expireAt,
    };

    const signature = await connectorClient.signer.signArbitrary(sessionInfo);

    const session: SigningSession = {
      privateKey: keyPair.privateKey,
      publicKey: keyPair.getPublicKey(),
      sessionInfo,
      sessionInfoSignature: signature,
    };

    setSession(session);
  }

  return {
    client,
    createSessionKey,
    session,
  };
}
