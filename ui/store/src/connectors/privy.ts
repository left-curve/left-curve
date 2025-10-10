import { decodeHex, encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";

import { createKeyHash, createSignerClient, toAccount } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";

import { createConnector } from "./createConnector.js";
import { composeArbitraryTypedData } from "@left-curve/dango/utils";

import type { Eip712Signature } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";
import type { EIP1193Provider } from "../types/eip1193.js";

const ETHEREUM_HEX_CHAIN_ID = "0x1";

type PrivyConnectorParameters = {
  icon?: string;
};

export function privy(parameters: PrivyConnectorParameters = {}) {
  const { icon } = parameters;

  return createConnector<EIP1193Provider>(({ transport, emitter, getUsername, chain }) => {
    return {
      id: "privy",
      name: "Privy",
      type: "privy",
      icon,
      async connect({ username, chainId, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "privy",
          username,
          transport,
        });

        const provider = await this.getProvider();
        await this.switchChain?.({ chainId: ETHEREUM_HEX_CHAIN_ID });
        const accountsInfo = await getAccountsByUsername(client, { username });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ username, address: address as Address, info: accountInfo }),
        );

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          return createKeyHash(controllerAddress.toLowerCase());
        })();

        const keys = await getKeysByUsername(client, { username });

        if (!keys[keyHash]) throw new Error("Not authorized");

        emitter.emit("connect", { accounts, chainId, username, keyHash });
      },
      async disconnect() {
        emitter.emit("disconnect");
        (window as any).privy.disconnect();
        (window as any).privy = undefined;
      },
      async getClient() {
        const username = getUsername();
        if (!username) throw new Error("privy: username not found");

        return createSignerClient({
          signer: this,
          type: "privy",
          chain,
          username,
          transport,
        });
      },
      async createNewKey(_challenge) {
        const provider = await this.getProvider();

        const [controllerAddress] = await provider.request({
          method: "eth_requestAccounts",
        });

        const addressLowerCase = controllerAddress.toLowerCase();

        const keyHash = createKeyHash(addressLowerCase);
        return { key: { ethereum: addressLowerCase as Address }, keyHash };
      },
      async getKeyHash() {
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const addressLowerCase = controllerAddress.toLowerCase();
        return createKeyHash(addressLowerCase);
      },
      async getProvider() {
        const provider: EIP1193Provider = await (window as any).privy.getEthereumProvider();
        return provider;
      },
      async getAccounts() {
        const client = await this.getClient();
        const username = getUsername();
        if (!username) throw new Error("eip1193: username not found");

        const accounts = await getAccountsByUsername(client, { username });
        return Object.entries(accounts).map(([address, accountInfo]) =>
          toAccount({ username, address: address as Address, info: accountInfo }),
        );
      },
      async switchChain({ chainId }) {
        const provider = await this.getProvider();

        await provider.request({
          method: "wallet_switchEthereumChain",
          params: [{ chainId }],
        });
      },
      async isAuthorized() {
        const provider = await this.getProvider();
        await this.switchChain?.({ chainId: ETHEREUM_HEX_CHAIN_ID });
        const [controllerAddress] = await provider.request({ method: "eth_accounts" });
        const accounts = await this.getAccounts();
        return !!controllerAddress && accounts.length > 0;
      },
      async signArbitrary(payload) {
        const { types, primaryType, message } = payload;

        const provider = await this.getProvider();
        await this.switchChain?.({ chainId: ETHEREUM_HEX_CHAIN_ID });
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        const typedData = composeArbitraryTypedData({ message, types, primaryType });
        const signData = JSON.stringify(typedData);

        const signature = await provider.request({
          method: "eth_signTypedData_v4",
          params: [controllerAddress, signData],
        });

        const eip712: Eip712Signature = {
          sig: encodeBase64(decodeHex(signature.slice(2))),
          typed_data: encodeBase64(encodeUtf8(signData)),
        };

        const keyHash = createKeyHash(controllerAddress.toLowerCase());

        return {
          credential: { standard: { keyHash, signature: { eip712 } } },
          signed: payload,
        };
      },
      async signTx(signDoc) {
        const provider = await this.getProvider();
        await this.switchChain?.({ chainId: ETHEREUM_HEX_CHAIN_ID });
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        const signData = JSON.stringify(signDoc);

        const signature = await provider.request({
          method: "eth_signTypedData_v4",
          params: [controllerAddress, signData],
        });

        const eip712: Eip712Signature = {
          sig: encodeBase64(decodeHex(signature.slice(2))),
          typed_data: encodeBase64(encodeUtf8(signData)),
        };

        const keyHash = createKeyHash(controllerAddress.toLowerCase());

        const standard = { signature: { eip712 }, keyHash };

        return { credential: { standard }, signed: signDoc };
      },
    };
  });
}
