import { createSignerClient, toAccount } from "@left-curve/sdk";
import { getUser } from "@left-curve/sdk/actions";

import { createConnector } from "./createConnector.js";

import type { Address, KeyHash } from "@left-curve/types";

export function debug() {
  let _keyHash_: KeyHash | undefined;

  return createConnector<undefined>(({ transport, emitter, getUserIndex, chain }) => {
    return {
      id: "debug",
      name: "Debug",
      icon: undefined,
      type: "debug",
      async connect({ userIndex, chainId }) {
        const client = createSignerClient({
          signer: this,
          type: "debug",
          chain,
          transport,
        });

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });

        const keyHashes = Object.keys(user.keys) as KeyHash[];
        if (keyHashes.length === 0) throw new Error("debug: user has no registered keys");
        const keyHash = keyHashes[0];
        _keyHash_ = keyHash;

        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );

        const account = accounts[0];
        const userStatus = await client.getAccountStatus({ address: account.address });

        emitter.emit("connect", {
          accounts,
          chainId,
          userIndex,
          keyHash,
          userStatus,
          username: user.name,
        });
      },
      async disconnect() {
        _keyHash_ = undefined;
        emitter.emit("disconnect");
      },
      async getClient() {
        return createSignerClient({
          signer: this,
          type: "debug",
          chain,
          transport,
        });
      },
      async getKeyHash() {
        if (!_keyHash_) throw new Error("debug: not connected");
        return _keyHash_;
      },
      async getAccounts() {
        const client = await this.getClient();
        const userIndex = getUserIndex();
        if (userIndex === undefined) throw new Error("debug: user index not found");
        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        return Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );
      },
      async isAuthorized() {
        return Boolean(_keyHash_);
      },
      async signArbitrary() {
        throw new Error("Debug connector: signing is disabled");
      },
      async signTx() {
        throw new Error("Debug connector: signing is disabled");
      },
    };
  });
}
