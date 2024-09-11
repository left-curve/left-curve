import { ethHashMessage, recoverPublicKey } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";
import { createKeyHash, createUserClient } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { composeAndHashTypedData } from "@leftcurve/utils";
import { createConnector } from "./createConnector";

import type { AccountTypes, Address, EIP1193Provider, Transport } from "@leftcurve/types";

import "@leftcurve/types/window";
import type { UserClient } from "@leftcurve/sdk/clients";
import { ConnectorSigner } from "@leftcurve/sdk/signers";

type EIP1193ConnectorParameters = {
  id?: string;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters = {}) {
  let _transport: Transport;
  let _username: string;
  let _client: UserClient;
  let _isAuthorized = false;

  const {
    id = "eip1193",
    name = "Ethereum Provider",
    provider: _provider_ = () => window.ethereum,
    icon,
  } = parameters;

  return createConnector<EIP1193Provider>(({ transports, emitter }) => {
    return {
      id,
      name,
      icon,
      type: "eip1193",
      async connect({ username, chainId, challenge }) {
        _username = username;
        _transport = transports[chainId];

        const client = await this.getClient();
        const provider = await this.getProvider();
        const accounts = await this.getAccounts();

        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        if (challenge) {
          const signature = await provider.request({
            method: "personal_sign",
            params: [challenge, controllerAddress],
          });

          const pubKey = await recoverPublicKey(ethHashMessage(challenge), signature, true);

          const keyHash = createKeyHash({ pubKey });
          const keys = await getKeysByUsername(client, { username });

          if (!keys[keyHash]) throw new Error("Not authorized");
          _isAuthorized = true;
        }

        emitter.emit("connect", { accounts, chainId, username });
      },
      async disconnect() {
        _isAuthorized = false;
        emitter.emit("disconnect");
      },
      async getClient() {
        if (!_client) {
          _client = createUserClient({
            transport: _transport,
            signer: new ConnectorSigner(this),
            username: _username,
          });
        }
        return _client;
      },
      async getKeyHash() {
        const provider = await this.getProvider();
        const challenge = encodeHex(crypto.getRandomValues(new Uint8Array(32)));
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        const signature = await provider.request({
          method: "personal_sign",
          params: [challenge, controllerAddress],
        });

        const pubKey = await recoverPublicKey(ethHashMessage(challenge), signature, true);

        return createKeyHash({ pubKey });
      },
      async getProvider() {
        const provider = _provider_();
        if (!provider) throw new Error(`${name} not detected`);
        return provider;
      },
      async getAccounts() {
        const client = await this.getClient();
        const accounts = await getAccountsByUsername(client, { username: _username });
        return Object.entries(accounts).map(([address, type]) => ({
          address: address as Address,
          username: _username,
          type: type as AccountTypes,
        }));
      },
      async isAuthorized() {
        return _isAuthorized;
      },
      async requestSignature(signDoc) {
        const { typedData, ...txMessage } = signDoc;
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        if (!typedData) throw new Error("Typed data required");
        const hashTypedData = composeAndHashTypedData(txMessage, typedData);

        const signature = await provider.request({
          method: "eth_signTypedData_v4",
          params: [controllerAddress, hashTypedData],
        });

        const ethWalletCredential = serialize({ signature, typedData: hashTypedData.substring(2) });
        const credential = { ethWallet: encodeBase64(ethWalletCredential) };

        const keyHash = createKeyHash({
          pubKey: await recoverPublicKey(hashTypedData.substring(2), signature, true),
        });

        return { credential, keyHash, signDoc };
      },
    };
  });
}
