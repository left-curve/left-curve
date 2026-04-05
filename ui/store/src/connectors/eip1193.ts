import { createKeyHash, createSignerClient, toAccount } from "@left-curve/dango";
import { getUser } from "@left-curve/dango/actions";
import { decodeHex, encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";
import { composeArbitraryTypedData } from "@left-curve/dango/utils";

import { createConnector } from "./createConnector.js";

import type { Eip712Signature } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";

import type { ConnectorId } from "../types/connector.js";
import type { EIP1193Provider } from "../types/eip1193.js";

const ETHEREUM_HEX_CHAIN_ID = "0x1";

type EIP1193ConnectorParameters = {
  id: ConnectorId;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters) {
  const {
    id = "metamask",
    name = "Ethereum Provider",
    provider: _provider_ = () => window.ethereum,
    icon,
  } = parameters;

  return createConnector<EIP1193Provider>(({ transport, getUserIndex, emitter, chain }) => {
    return {
      id,
      name,
      icon,
      type: "eip1193",
      async connect({ userIndex, chainId, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "eip1193",
          transport,
        });

        const provider = await this.getProvider();
        await this.switchChain?.({ chainId: ETHEREUM_HEX_CHAIN_ID });

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          return createKeyHash(controllerAddress.toLowerCase());
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
          type: "eip1193",
          chain,
          transport,
        });
      },
      async createNewKey(_challenge = "Please sign this message to confirm your identity.") {
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
        const userIndex = getUserIndex();
        if (userIndex === undefined) throw new Error("eip1193: user index not found");

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        return Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
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
