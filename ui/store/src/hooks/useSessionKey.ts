import { createSessionSigner, createSignerClient } from "@left-curve/dango";
import { Secp256k1 } from "@left-curve/dango/crypto";
import { createStorage, useStorage } from "@left-curve/foundation";

import { useQuery } from "@tanstack/react-query";
import { useAccount } from "./useAccount.js";
import { useConfig } from "./useConfig.js";

import { encodeBase64 } from "@left-curve/dango/encoding";
import type { SigningSession, SigningSessionInfo } from "@left-curve/dango/types";
import type { Connector } from "../types/connector.js";
import { useEffect, useState } from "react";

export type UseSessionKeyParameters = {
  session?: SigningSession;
};

type CreateSessionKeyParameters = {
  connector?: Connector;
  expireAt: number;
};

type CreateSessionKeyOptions = {
  setSession?: boolean;
};

export type UseSessionKeyReturnType = {
  client?: ReturnType<typeof createSignerClient> | null;
  session: SigningSession | null;
  setSession: (session: SigningSession | null) => void;
  deleteSessionKey: () => void;
  createSessionKey: (
    parameters: CreateSessionKeyParameters,
    options?: CreateSessionKeyOptions,
  ) => Promise<SigningSession>;
};

export function useSessionKey(parameters: UseSessionKeyParameters = {}): UseSessionKeyReturnType {
  const config = useConfig();
  const { username, connector } = useAccount();

  const [channel] = useState(new BroadcastChannel("dango.session"));

  const [session, setSession] = useStorage<SigningSession | null>("session_key", {
    initialValue: parameters.session,
    storage: createStorage({ storage: window?.sessionStorage }),
    version: 1.1,
  });

  const { data: client } = useQuery({
    enabled: Boolean(session) && Boolean(username),
    queryKey: ["session_key", username, session?.keyHash],
    queryFn: async () => {
      if (!session || !username) return null;
      return createSignerClient({
        username,
        chain: config.chain,
        signer: createSessionSigner(session),
        transport: config._internal.transport,
      });
    },
  });

  useEffect(() => {
    if (!session) channel.postMessage({ type: "request" });
  }, []);

  useEffect(() => {
    function handleMessage({ data: event }: MessageEvent) {
      if (event.type === "request" && session) {
        channel.postMessage({ type: "response", data: session });
      }

      if (event.type === "response") {
        setSession(event.data);
      }
    }
    channel.addEventListener("message", handleMessage);

    return () => {
      channel.removeEventListener("message", handleMessage);
    };
  }, [session]);

  async function createSessionKey(
    parameters: CreateSessionKeyParameters,
    options: CreateSessionKeyOptions = {},
  ) {
    const c = parameters.connector || connector;
    if (!c) throw new Error("connector not found");
    const { expireAt } = parameters;
    const { setSession: loadSession = true } = options;

    const keyPair = Secp256k1.makeKeyPair();
    const publicKey = keyPair.getPublicKey();

    const sessionInfo: SigningSessionInfo = {
      sessionKey: encodeBase64(publicKey),
      expireAt: expireAt.toString(),
    };

    const { credential } = await c.signArbitrary({
      primaryType: "Message" as const,
      message: sessionInfo,
      types: {
        Message: [
          { name: "session_key", type: "string" },
          { name: "expire_at", type: "string" },
        ],
      },
    });

    if (!("standard" in credential)) throw new Error("unsupported credential type");

    const session = {
      keyHash: credential.standard.keyHash,
      sessionInfo,
      privateKey: keyPair.privateKey,
      publicKey,
      authorization: credential.standard,
    };

    if (loadSession) setSession(session);

    return session;
  }

  function deleteSessionKey() {
    setSession(null);
  }

  return {
    client,
    session,
    setSession,
    deleteSessionKey,
    createSessionKey,
  };
}
