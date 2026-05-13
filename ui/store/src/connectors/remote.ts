import { createSignerClient, toAccount } from "@left-curve/dango";
import { getUser } from "@left-curve/dango/actions";

import { createConnector } from "./createConnector.js";
import { requestRemote } from "../remote.js";

import type { KeyHash, ArbitrarySignatureOutcome, SignatureOutcome } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";

export function remote() {
  return createConnector<undefined>(({ transport, getUserIndex, emitter, chain }) => {
    return {
      id: "remote",
      name: "Remote",
      icon: undefined,
      type: "remote",
      async connect({ userIndex, chainId, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "remote",
          transport,
        });

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          return await this.getKeyHash();
        })();

        if (!user.keys[keyHash]) throw new Error("Not authorized");

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
        emitter.emit("disconnect");
      },
      async getClient() {
        return createSignerClient({
          signer: this,
          type: "remote",
          chain,
          transport,
        });
      },
      async createNewKey(_challenge = "Please sign this message to confirm your identity.") {
        return await requestRemote<{ key: { ethereum: Address }; keyHash: KeyHash }>(
          "connector",
          "createNewKey",
        );
      },
      async getKeyHash() {
        return await requestRemote<KeyHash>("connector", "getKeyHash");
      },
      async getAccounts() {
        const client = await this.getClient();
        const userIndex = getUserIndex();
        if (userIndex === undefined) throw new Error("remote: user index not found");

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        return Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );
      },
      async isAuthorized() {
        return true;
      },
      async signArbitrary(payload) {
        return await requestRemote<ArbitrarySignatureOutcome>(
          "connector",
          "signArbitrary",
          payload,
        );
      },
      async signTx(signDoc) {
        return await requestRemote<SignatureOutcome>("connector", "signTx", signDoc);
      },
    };
  });
}
