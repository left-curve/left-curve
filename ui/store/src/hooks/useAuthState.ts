import { useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";

import { createKeyHash } from "@left-curve/dango";
import { registerUser } from "@left-curve/dango/actions";

import { useSessionKey } from "./useSessionKey.js";
import { useConnectors } from "./useConnectors.js";
import { usePublicClient } from "./usePublicClient.js";
import { useChainId } from "./useChainId.js";
import { useConfig } from "./useConfig.js";

import type { Address, Key, KeyHash, SigningSession, User } from "@left-curve/dango/types";
import type { EIP1193Provider } from "../types/eip1193.js";
import type { Connector } from "../types/connector.js";

export type AuthScreen =
  | "options"
  | "email"
  | "wallets"
  | "passkey-choice"
  | "passkey-error"
  | "create-account"
  | "account-picker"
  | "deposit";

export type UseAuthStateParameters = {
  session: boolean;
  expiration: number;
  referrer?: number;
  onSuccess?: () => void;
  onError?: (e: unknown) => void;
};

type AuthData = {
  users: User[];
  keyHash?: KeyHash;
  key?: Key;
  signingSession?: SigningSession;
  connectorId?: string;
  identifier?: string;
};

export function useAuthState(parameters: UseAuthStateParameters) {
  const { session, expiration, onSuccess, onError } = parameters;

  const { createSessionKey, setSession } = useSessionKey();
  const connectors = useConnectors();
  const publicClient = usePublicClient();
  const chainId = useChainId();
  const config = useConfig();

  const [screen, setScreen] = useState<AuthScreen>("options");
  const [email, setEmail] = useState("");
  const [referrer, setReferrer] = useState<number | undefined>(parameters.referrer);

  const connectorRef = useRef<Connector | null>(null);
  const [authData, setAuthData] = useState<AuthData>({ users: [] });

  const authenticate = useMutation({
    onError,
    mutationFn: async (connectorId: string) => {
      const connector = connectors.find((c) => c.id === connectorId);
      if (!connector) throw new Error("error: missing connector");
      connectorRef.current = connector;

      if (connectorId === "passkey") {
        setAuthData((prev) => ({ ...prev, connectorId }));
        setScreen("passkey-choice");
        return;
      }

      // For wallet/email/social: get key info, then try login
      let keyHash: KeyHash;
      let key: Key | undefined;
      let signingSession: SigningSession | undefined;
      let identifier: string | undefined;

      if (session) {
        signingSession = await createSessionKey(
          { connector, expireAt: Date.now() + expiration },
          { setSession: false },
        );
        keyHash = signingSession.keyHash;
      } else {
        // For wallet connectors, get address as identifier
        if (connectorId !== "privy") {
          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();
          const [controllerAddress] = await provider.request({
            method: "eth_requestAccounts",
          });
          identifier = controllerAddress;
        }

        keyHash = await connector.getKeyHash();
      }

      const users = await publicClient.forgotUsername({ keyHash });

      if (users.length === 0) {
        // No account found — need to register
        // Get key for registration if we don't have it yet
        if (!key && connectorId !== "privy") {
          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();
          const [controllerAddress] = await provider.request({
            method: "eth_requestAccounts",
          });
          const addressLowerCase = controllerAddress.toLowerCase() as Address;
          key = { ethereum: addressLowerCase } as Key;
          identifier = controllerAddress;
          keyHash = createKeyHash(addressLowerCase);
        }

        if (!identifier) {
          identifier = email || keyHash;
        }

        setAuthData({ users: [], connectorId, keyHash, key, signingSession, identifier });
        setScreen("create-account");
        return;
      }

      if (users.length === 1) {
        // Single account — auto-login
        setAuthData({ users, connectorId, keyHash, signingSession });
        await loginUser(users[0].index, connector, keyHash, signingSession);
        return;
      }

      // Multiple accounts — show picker
      setAuthData({ users, connectorId, keyHash, key, signingSession, identifier });
      setScreen("account-picker");
    },
  });

  const passkeyCreate = useMutation({
    onError,
    mutationFn: async () => {
      const connector = connectorRef.current;
      if (!connector) throw new Error("error: missing connector");

      const challenge = "Please sign this message to confirm your identity.";
      const { key, keyHash } = await connector.createNewKey!(challenge);

      setAuthData((prev) => ({
        ...prev,
        key,
        keyHash,
        identifier: "Passkey",
      }));
      setScreen("create-account");
    },
  });

  const passkeyLogin = useMutation({
    onError,
    mutationFn: async () => {
      const connector = connectorRef.current;
      if (!connector) throw new Error("error: missing connector");

      let keyHash: KeyHash;
      let signingSession: SigningSession | undefined;

      if (session) {
        signingSession = await createSessionKey(
          { connector, expireAt: Date.now() + expiration },
          { setSession: false },
        );
        keyHash = signingSession.keyHash;
      } else {
        keyHash = await connector.getKeyHash();
      }

      const users = await publicClient.forgotUsername({ keyHash });

      if (users.length === 0) {
        setAuthData((prev) => ({ ...prev, keyHash, signingSession }));
        setScreen("passkey-error");
        return;
      }

      if (users.length === 1) {
        setAuthData({ users, connectorId: "passkey", keyHash, signingSession });
        await loginUser(users[0].index, connector, keyHash, signingSession);
        return;
      }

      setAuthData({ users, connectorId: "passkey", keyHash, signingSession });
      setScreen("account-picker");
    },
  });

  const createAccount = useMutation({
    onError,
    mutationFn: async () => {
      const connector = connectorRef.current;
      if (!connector) throw new Error("error: missing connector");

      const connectorId = authData.connectorId;
      let { key, keyHash } = authData;

      // If we don't have key/keyHash yet (e.g. privy flow), get them now
      if (!key || !keyHash) {
        if (connectorId === "passkey") {
          const result = await connector.createNewKey!();
          key = result.key;
          keyHash = result.keyHash;
        } else if (connectorId === "privy") {
          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();
          const [controllerAddress] = await provider.request({
            method: "eth_requestAccounts",
          });
          const addressLowerCase = controllerAddress.toLowerCase() as Address;
          key = { ethereum: addressLowerCase } as Key;
          keyHash = createKeyHash(addressLowerCase);
        } else {
          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();
          const [controllerAddress] = await provider.request({
            method: "eth_requestAccounts",
          });
          const addressLowerCase = controllerAddress.toLowerCase() as Address;
          key = { ethereum: addressLowerCase } as Key;
          keyHash = createKeyHash(addressLowerCase);
        }
      }

      const { credential } = await connector.signArbitrary({
        primaryType: "Message" as const,
        message: { chain_id: config.chain.id },
        types: { Message: [{ name: "chain_id", type: "string" }] },
      });

      if (!("standard" in credential)) throw new Error("Signed with wrong credential");

      await registerUser(publicClient, {
        key,
        keyHash,
        seed: Math.floor(Math.random() * 0x100000000),
        signature: credential.standard.signature,
        referrer,
      });

      // After registration, login
      if (session) {
        const signingSession = await createSessionKey(
          { connector, expireAt: Date.now() + expiration },
          { setSession: false },
        );

        const users = await publicClient.forgotUsername({
          keyHash: signingSession.keyHash,
        });

        const userIndex = users[users.length - 1].index;
        setSession(signingSession);

        await connector.connect({
          userIndex,
          chainId,
          keyHash: signingSession.keyHash,
        });
      } else {
        const resolvedKeyHash = await connector.getKeyHash();
        const users = await publicClient.forgotUsername({ keyHash: resolvedKeyHash });
        const userIndex = users[users.length - 1].index;

        await connector.connect({
          userIndex,
          chainId,
          keyHash: resolvedKeyHash,
        });
      }

      onSuccess?.();
    },
  });

  const selectAccount = useMutation({
    onError,
    mutationFn: async (userIndex: number) => {
      const connector = connectorRef.current;
      if (!connector) throw new Error("error: missing connector");
      await loginUser(userIndex, connector, authData.keyHash, authData.signingSession);
    },
  });

  const createNewWithExistingKey = useMutation({
    onError,
    mutationFn: async () => {
      const connector = connectorRef.current;
      if (!connector) throw new Error("error: missing connector");

      const connectorId = authData.connectorId;
      let key: Key;
      let keyHash: KeyHash;

      if (connectorId === "passkey") {
        const result = await connector.createNewKey!();
        key = result.key;
        keyHash = result.keyHash;
      } else {
        const provider = await (
          connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
        ).getProvider();
        const [controllerAddress] = await provider.request({
          method: "eth_requestAccounts",
        });
        const addressLowerCase = controllerAddress.toLowerCase() as Address;
        key = { ethereum: addressLowerCase } as Key;
        keyHash = createKeyHash(addressLowerCase);
      }

      setAuthData((prev) => ({ ...prev, key, keyHash, identifier: "New account" }));
      setScreen("create-account");
    },
  });

  async function loginUser(
    userIndex: number,
    connector: Connector,
    keyHash?: KeyHash,
    signingSession?: SigningSession,
  ) {
    if (signingSession) {
      setSession(signingSession);
      await connector.connect({
        userIndex,
        chainId,
        keyHash: signingSession.keyHash,
      });
    } else {
      await connector.connect({
        userIndex,
        chainId,
        ...(keyHash
          ? { keyHash }
          : { challenge: "Please sign this message to confirm your identity." }),
      });
    }

    onSuccess?.();
  }

  const isPending =
    authenticate.isPending ||
    passkeyCreate.isPending ||
    passkeyLogin.isPending ||
    createAccount.isPending ||
    selectAccount.isPending ||
    createNewWithExistingKey.isPending;

  return {
    screen,
    setScreen,
    email,
    setEmail,
    referrer,
    setReferrer,
    users: authData.users,
    identifier: authData.identifier,
    authenticate,
    passkeyCreate,
    passkeyLogin,
    createAccount,
    selectAccount,
    createNewWithExistingKey,
    isPending,
  };
}
