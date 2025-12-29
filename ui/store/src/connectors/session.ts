import { createSessionSigner, createSignerClient, toAccount } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";
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

  return createConnector<SigningSession>(({ transport, emitter, getUserIndexAndName, chain }) => {
    return {
      id,
      name,
      icon,
      type: "session",
      async setup() {
        _provider_ = parameters.target?.provider || (async () => await storage.getItem("session"));
      },
      async connect({ userIndexAndName, chainId, challenge }) {
        const client = createSignerClient({
          signer: this,
          type: "session",
          transport,
        });

        if (!challenge) throw new Error("challenge is required to recover the session");

        const session = deserializeJson<SigningSession>(decodeUtf8(decodeBase64(challenge)));
        const keys = await getKeysByUsername(client, { userIndexOrName: userIndexAndName });

        if (!keys[session.keyHash]) throw new Error("Not authorized");
        storage.setItem("session", session);
        const accountsInfo = await getAccountsByUsername(client, {
          userIndexOrName: userIndexAndName,
        });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
        );

        const account = accounts[0];
        const userStatus = await client.getAccountStatus({ address: account.address });

        emitter.emit("connect", {
          accounts,
          chainId,
          userIndexAndName,
          keyHash: session.keyHash,
          userStatus,
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
        const userIndexAndName = await getUserIndexAndName();
        if (!userIndexAndName) throw new Error("session: user index not found");
        const accountsInfo = await getAccountsByUsername(client, {
          userIndexOrName: userIndexAndName,
        });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
        );
        return accounts;
      },
      async isAuthorized() {
        const accounts = await this.getAccounts();
        const session = await storage.getItem<"session", SigningSession, undefined>("session");
        const isExpired = Number(session?.sessionInfo.expireAt || 0) < Date.now();
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
