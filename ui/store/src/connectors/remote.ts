import { createSignerClient, toAccount } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";

import { createConnector } from "./createConnector.js";
import { requestRemote } from "../remote.js";

import type { KeyHash, ArbitrarySignatureOutcome, SignatureOutcome } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";

export function remote() {
  return createConnector<undefined>(({ transport, getUserIndexAndName, emitter, chain }) => {
    return {
      id: "remote",
      name: "Remote",
      icon: undefined,
      type: "remote",
      async connect({ userIndexAndName, chainId, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "remote",
          transport,
        });

        const accountsInfo = await getAccountsByUsername(client, {
          userIndexOrName: userIndexAndName,
        });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
        );

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          return await this.getKeyHash();
        })();

        const keys = await getKeysByUsername(client, { userIndexOrName: userIndexAndName });

        if (!keys[keyHash]) throw new Error("Not authorized");

        emitter.emit("connect", { accounts, chainId, userIndexAndName, keyHash });
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
        const userIndexAndName = await getUserIndexAndName();
        if (!userIndexAndName) throw new Error("remote: user index not found");

        const accounts = await getAccountsByUsername(client, { userIndexOrName: userIndexAndName });
        return Object.entries(accounts).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
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
