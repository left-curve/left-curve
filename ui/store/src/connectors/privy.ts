import { decodeHex, encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";

import { createKeyHash, createSignerClient, toAccount } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";

import { composeArbitraryTypedData } from "@left-curve/dango/utils";
import { createConnector } from "./createConnector.js";

import Privy, {
  getEntropyDetailsFromUser,
  getUserEmbeddedEthereumWallet,
  LocalStorage,
} from "@privy-io/js-sdk-core";

import type { Eip712Signature } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";
import type { EIP1193Provider } from "../types/eip1193.js";

const ETHEREUM_HEX_CHAIN_ID = "0x1";

type MessagePoster = {
  reload: () => void;
  postMessage: (message: unknown, targetOrigin: string, transfer?: Transferable) => void;
};

type PrivyConnectorParameters = {
  icon?: string;
  appId: string;
  clientId: string;
  poster: (url: string) => MessagePoster;
  listener: (callback: (data: any) => void) => void;
};

export function privy(parameters: PrivyConnectorParameters) {
  const { appId, clientId, poster, listener, icon } = parameters;

  const privy = new Privy({
    appId,
    clientId,
    storage: new LocalStorage(),
  });

  return createConnector<EIP1193Provider>(({ transport, emitter, getUserIndexAndName, chain }) => {
    return {
      id: "privy",
      name: "Privy",
      type: "privy",
      icon,
      privy,
      async setup() {
        privy.setMessagePoster(poster(privy.embeddedWallet.getURL()));
        listener((data) => privy.embeddedWallet.onMessage(data));

        await privy.initialize();
      },
      async connect({ userIndexAndName, chainId, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "privy",
          transport,
        });

        const provider = await this.getProvider();
        await this.switchChain?.({ chainId: ETHEREUM_HEX_CHAIN_ID });
        const accountsInfo = await getAccountsByUsername(client, {
          userIndexOrName: userIndexAndName,
        });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
        );

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          return createKeyHash(controllerAddress.toLowerCase());
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
          type: "privy",
          chain,
          transport,
        });
      },
      async getKeyHash() {
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const addressLowerCase = controllerAddress.toLowerCase();
        return createKeyHash(addressLowerCase);
      },
      async getProvider() {
        const { user } = await privy.user.get();
        if (!user) throw new Error("we couldn't recover the session");
        const wallet = getUserEmbeddedEthereumWallet(user)!;
        const { entropyId, entropyIdVerifier } = getEntropyDetailsFromUser(user)!;

        return (await privy.embeddedWallet.getEthereumProvider({
          wallet,
          entropyId,
          entropyIdVerifier,
        })) as unknown as EIP1193Provider;
      },
      async getAccounts() {
        const client = await this.getClient();
        const userIndexAndName = await getUserIndexAndName();
        if (!userIndexAndName) throw new Error("eip1193: user index not found");

        const accountsInfo = await getAccountsByUsername(client, {
          userIndexOrName: userIndexAndName,
        });
        return Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
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
