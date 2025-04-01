import { createKeyHash, createSignerClient, toAccount } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";
import { decodeHex, encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";
import { composeArbitraryTypedData } from "@left-curve/dango/utils";

import { createConnector } from "./createConnector.js";

import type { Eip712Signature } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";

import type { ConnectorId } from "../types/connector.js";
import type { EIP1193Provider } from "../types/eip1193.js";

type EIP1193ConnectorParameters = {
  id: ConnectorId;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters) {
  let _isAuthorized = false;

  const {
    id = "metamask",
    name = "Ethereum Provider",
    provider: _provider_ = () => window.ethereum,
    icon,
  } = parameters;

  return createConnector<EIP1193Provider>(({ transport, getUsername, emitter }) => {
    return {
      id,
      name,
      icon,
      type: "eip1193",
      async connect({ username, chainId, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "eip1193",
          username,
          transport,
        });

        const provider = await this.getProvider();
        const accountsInfo = await getAccountsByUsername(client, { username });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ username, address: address as Address, info: accountInfo }),
        );

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          return createKeyHash(controllerAddress);
        })();

        const keys = await getKeysByUsername(client, { username });

        if (!keys[keyHash]) throw new Error("Not authorized");

        emitter.emit("connect", { accounts, chainId, username, keyHash });
      },
      async disconnect() {
        _isAuthorized = false;
        emitter.emit("disconnect");
      },
      async getClient() {
        const username = getUsername();
        if (!username) throw new Error("eip1193: username not found");

        return createSignerClient({
          signer: this,
          type: "eip1193",
          username,
          transport,
        });
      },
      async createNewKey(challenge = "Please sign this message to confirm your identity.") {
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
        const provider = _provider_();
        if (!provider) throw new Error(`${name} not detected`);
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
      async isAuthorized() {
        return _isAuthorized;
      },
      async signArbitrary(payload) {
        const { types, primaryType, message } = payload;

        const provider = await this.getProvider();
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
        try {
          const provider = await this.getProvider();
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
        } catch (error) {
          console.error(error);
          throw error;
        }
      },
    };
  });
}
