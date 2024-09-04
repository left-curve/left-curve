import { ethHashMessage, recoverPublicKey, ripemd160 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, encodeUtf8 } from "@leftcurve/encoding";
import { createBaseClient } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { Client, EIP1193Provider, KeyHash, Transport } from "@leftcurve/types";

import "@leftcurve/types/window";

type EIP1193ConnectorParameters = {
  id?: string;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters = {}) {
  let _transport: Transport;
  let _username: string;
  let _client: Client;
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
        await this.getClient();
        const accounts = await this.getAccounts();
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        if (challenge) {
          const signature = await provider.request({
            method: "personal_sign",
            params: [challenge, controllerAddress],
          });
          const hashMessage = ethHashMessage(challenge);

          const publicKey = await recoverPublicKey(hashMessage, signature, true);

          const keyHash: KeyHash = encodeHex(ripemd160(publicKey)).toUpperCase();
          const keys = await getKeysByUsername(_client, { username });

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
        if (!_client) _client = createBaseClient({ transport: _transport });
        return _client;
      },
      async getProvider() {
        const provider = _provider_();
        if (!provider) throw new Error(`${name} not detected`);
        return provider;
      },
      async getAccounts() {
        const accounts = await getAccountsByUsername(_client, { username: _username });
        return Object.entries(accounts).map(([index, info]) => ({
          id: `${_username}/account/${Number(index)}`,
          index: Number(index),
          username: _username,
          ...info,
        }));
      },
      async isAuthorized() {
        return _isAuthorized;
      },
      async requestSignature(typedData) {
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const signature = await provider.request({
          method: "eth_signTypedData_v4",
          params: [controllerAddress, JSON.stringify(typedData)],
        });
        const credential = encodeUtf8(JSON.stringify({ signature, typed_data: typedData }));
        return { walletEvm: encodeBase64(credential) };
      },
    };
  });
}
