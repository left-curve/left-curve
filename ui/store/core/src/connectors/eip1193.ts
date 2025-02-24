import { createKeyHash, createSignerClient } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";
import { ethHashMessage, secp256k1RecoverPubKey } from "@left-curve/dango/crypto";
import { decodeHex, encodeBase64, encodeHex, encodeUtf8 } from "@left-curve/dango/encoding";
import { KeyAlgo } from "@left-curve/dango/types";
import {
  composeArbitraryTypedData,
  composeTxTypedData,
  getRootDomain,
  hashTypedData,
} from "@left-curve/dango/utils";

import { createConnector } from "./createConnector.js";

import type { AccountTypes, Eip712Signature, SignerClient } from "@left-curve/dango/types";
import type { Address, Json, Transport, TypedDataProperty } from "@left-curve/dango/types";

import type { ConnectorId } from "../types/connector.js";
import type { EIP1193Provider } from "../types/eip1193.js";

type EIP1193ConnectorParameters = {
  id: ConnectorId;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters) {
  let _transport: Transport;
  let _username: string;
  let _client: SignerClient;
  let _isAuthorized = false;

  const {
    id = "metamask",
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
      async connect({ username, chainId, challenge, keyHash: _keyHash_ }) {
        _username = username;
        _transport = transports[chainId];

        const client = await this.getClient();
        const provider = await this.getProvider();
        const accounts = await this.getAccounts();

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          const c = challenge as string;
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          const signature = await provider.request({
            method: "personal_sign",
            params: [c, controllerAddress],
          });

          const pubKey = await secp256k1RecoverPubKey(ethHashMessage(c), signature, true);

          return createKeyHash({ pubKey, keyAlgo: KeyAlgo.Secp256k1 });
        })();

        const keys = await getKeysByUsername(client, { username });

        if (!keys[keyHash]) throw new Error("Not authorized");
        _isAuthorized = true;

        emitter.emit("connect", { accounts, chainId, username, keyHash });
      },
      async disconnect() {
        _isAuthorized = false;
        emitter.emit("disconnect");
      },
      async getClient() {
        if (!_client) {
          _client = createSignerClient({
            signer: this,
            type: "eip1193",
            username: _username,
            transport: _transport,
          });
        }
        return _client;
      },
      async createNewKey(challenge = "Please sign this message to confirm your identity.") {
        const provider = await this.getProvider();

        const [controllerAddress] = await provider.request({
          method: "eth_requestAccounts",
        });

        const signature = await provider.request({
          method: "personal_sign",
          params: [challenge, controllerAddress],
        });

        const pubKey = await secp256k1RecoverPubKey(ethHashMessage(challenge), signature, true);
        const keyHash = createKeyHash({ pubKey, keyAlgo: KeyAlgo.Secp256k1 });
        return { key: { secp256k1: encodeBase64(pubKey) }, keyHash };
      },
      async getKeyHash() {
        const provider = await this.getProvider();
        const challenge = encodeHex(crypto.getRandomValues(new Uint8Array(32)));
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        const signature = await provider.request({
          method: "personal_sign",
          params: [challenge, controllerAddress],
        });

        const pubKey = await secp256k1RecoverPubKey(ethHashMessage(challenge), signature, true);

        return createKeyHash({ pubKey, keyAlgo: KeyAlgo.Secp256k1 });
      },
      async getProvider() {
        const provider = _provider_();
        if (!provider) throw new Error(`${name} not detected`);
        return provider;
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
        const { types, primaryType, message } = payload as {
          types: Record<string, TypedDataProperty[]>;
          message: Json;
          primaryType: string;
        };
        if (!types || !primaryType) throw new Error("Typed data required");

        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

        const typedData = composeArbitraryTypedData({ message, types, primaryType });
        const hashData = await hashTypedData(typedData);
        const signData = JSON.stringify(typedData);

        const signature = await provider.request({
          method: "eth_signTypedData_v4",
          params: [controllerAddress, signData],
        });

        const eip712: Eip712Signature = {
          sig: encodeBase64(decodeHex(signature.slice(2).substring(0, 128))),
          typed_data: encodeBase64(encodeUtf8(signData)),
        };

        const keyHash = createKeyHash({
          pubKey: await secp256k1RecoverPubKey(hashData, signature, true),
          keyAlgo: KeyAlgo.Secp256k1,
        });

        return {
          credential: { standard: { keyHash, signature: { eip712 } } },
          payload,
        };
      },
      async signTx(signDoc, extra) {
        const { typedData: types } = extra as { typedData?: Record<string, TypedDataProperty[]> };
        try {
          const { sender, messages, gasLimit: gas_limit, data: metadata } = signDoc;
          const provider = await this.getProvider();
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          if (!types) throw new Error("Typed data required");

          const domain = {
            name: getRootDomain(window.location.hostname),
            verifyingContract: sender,
          };

          const typedData = composeTxTypedData({ messages, gas_limit, metadata }, domain, types);
          const hashData = await hashTypedData(typedData);
          const signData = JSON.stringify(typedData);

          const signature = await provider.request({
            method: "eth_signTypedData_v4",
            params: [controllerAddress, signData],
          });

          const eip712: Eip712Signature = {
            sig: encodeBase64(decodeHex(signature.slice(2).substring(0, 128))),
            typed_data: encodeBase64(encodeUtf8(signData)),
          };

          const keyHash = createKeyHash({
            pubKey: await secp256k1RecoverPubKey(hashData, signature, true),
            keyAlgo: KeyAlgo.Secp256k1,
          });

          const standard = { signature: { eip712 }, keyHash };

          return { credential: { standard }, signDoc };
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
