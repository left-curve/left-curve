import { createSessionSigner, createSignerClient } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";
import { decodeBase64, decodeUtf8, deserializeJson } from "@left-curve/dango/encoding";

import { createConnector } from "./createConnector.js";

import type { AccountTypes, SignerClient, SigningSession } from "@left-curve/dango/types";
import type { Address, Transport } from "@left-curve/dango/types";

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
  let _transport: Transport;
  let _username: string;
  let _client: SignerClient;
  let _isAuthorized = false;
  let _provider_ = async (): Promise<SigningSession | null> => await storage.getItem("session");

  const { storage = createStorage({ storage: sessionStorage }), target } = parameters;

  const { id = "session", name = "Session Provider", icon } = target || {};

  return createConnector<SigningSession>(({ transports, emitter }) => {
    return {
      id,
      name,
      icon,
      type: "session",
      async setup() {
        _provider_ = parameters.target?.provider || (async () => await storage.getItem("session"));
      },
      async connect({ username, chainId, challenge }) {
        _username = username;
        _transport = transports[chainId];

        const client = await this.getClient();

        if (!challenge) throw new Error("challenge is requiered to recover the session");

        const session = deserializeJson<SigningSession>(decodeUtf8(decodeBase64(challenge)));
        const keys = await getKeysByUsername(client, { username });

        if (!keys[session.keyHash]) throw new Error("Not authorized");
        _isAuthorized = true;
        storage.setItem("session", session);
        const accounts = await this.getAccounts();

        emitter.emit("connect", { accounts, chainId, username, keyHash: session.keyHash });
      },
      async disconnect() {
        _isAuthorized = false;
        storage.removeItem("session");
        emitter.emit("disconnect");
      },
      async getClient() {
        if (!_client) {
          _client = createSignerClient({
            signer: this,
            type: "session",
            username: _username,
            transport: _transport,
          });
        }
        return _client;
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
        const accounts = await getAccountsByUsername(client, { username: _username });
        return Object.entries(accounts).map(([address, accountInfo]) => {
          const { index, params } = accountInfo;
          const type = Object.keys(params)[0] as AccountTypes;
          return {
            index,
            params,
            address: address as Address,
            username: _username,
            type: type,
          };
        });
      },
      async isAuthorized() {
        return _isAuthorized;
      },
      async signArbitrary(payload) {
        const provider = await this.getProvider();
        const signer = createSessionSigner(provider);

        return await signer.signArbitrary(payload);
      },
      async signTx(signDoc) {
        try {
          const provider = await this.getProvider();
          const signer = createSessionSigner(provider);

          return await signer.signTx(signDoc);
        } catch (error) {
          console.error(error);
          throw error;
        }
      },
      onConnect({ chainId, username }) {
        _username = username;
        _transport = transports[chainId];
      },
    };
  });
}
