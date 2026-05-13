import { createSessionSigner, createSignerClient, toAccount } from "@left-curve/dango";
import { getUser } from "@left-curve/dango/actions";
import { decodeBase64, decodeUtf8, deserializeJson } from "@left-curve/dango/encoding";

import { createConnector } from "./createConnector.js";

import type { SigningSession } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";

import { createStorage } from "../storages/createStorage.js";
import type { Storage } from "../types/storage.js";

type SessionConnectorParameters = {
  storage?: Storage;
  target?: {
    id?: string;
    name?: string;
    icon?: string;
    provider?: () => Promise<SigningSession | null>;
  };
};

export function session(parameters: SessionConnectorParameters = {}) {
  let _provider_ = async (): Promise<SigningSession | null> => await storage.getItem("session");

  const { storage = createStorage({ storage: window?.sessionStorage }), target } = parameters;

  const { id = "session", name = "Session Provider", icon } = target || {};

  return createConnector<SigningSession>(({ transport, emitter, getUserIndex, chain }) => {
    return {
      id,
      name,
      icon,
      type: "session",
      async setup() {
        _provider_ = parameters.target?.provider || (async () => await storage.getItem("session"));
      },
      async connect({ userIndex, chainId, challenge }) {
        const client = createSignerClient({
          signer: this,
          type: "session",
          transport,
        });

        if (!challenge) throw new Error("challenge is required to recover the session");

        const session = deserializeJson<SigningSession>(decodeUtf8(decodeBase64(challenge)));

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });

        if (!user.keys[session.keyHash]) throw new Error("Not authorized");
        storage.setItem("session", session);

        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );

        const account = accounts[0];
        const userStatus = await client.getAccountStatus({ address: account.address });

        emitter.emit("connect", {
          accounts,
          chainId,
          userIndex,
          keyHash: session.keyHash,
          userStatus,
          username: user.name,
        });
      },
      async disconnect() {
        storage.removeItem("session");
        emitter.emit("disconnect");
      },
      async getClient() {
        return createSignerClient({
          signer: this,
          chain,
          type: "session",
          transport: transport,
        });
      },
      async getKeyHash() {
        const provider = await this.getProvider();
        return provider.keyHash;
      },
      async getProvider() {
        const session = await _provider_();
        if (!session) throw new Error(`${name} not detected`);
        return session;
      },
      async getAccounts() {
        const client = await this.getClient();
        const userIndex = getUserIndex();
        if (userIndex === undefined) throw new Error("session: user index not found");
        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );
        return accounts;
      },
      async isAuthorized() {
        const accounts = await this.getAccounts();
        const session = await storage.getItem<"session", SigningSession, undefined>("session");
        const isExpired = Number(session?.sessionInfo.expireAt || 0) * 1000 < Date.now();
        return !isExpired && accounts.length > 0;
      },
      async signArbitrary(payload) {
        const provider = await this.getProvider();
        const signer = createSessionSigner(provider);

        return await signer.signArbitrary(payload);
      },
      async signTx(signDoc) {
        const provider = await this.getProvider();
        const signer = createSessionSigner(provider);

        return await signer.signTx(signDoc);
      },
    };
  });
}
