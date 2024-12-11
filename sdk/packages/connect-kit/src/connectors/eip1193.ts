import { ethHashMessage, secp256k1RecoverPubKey } from "@left-curve/crypto";
import { decodeHex, encodeBase64, encodeHex, encodeUtf8 } from "@left-curve/encoding";
import { ConnectorSigner, createKeyHash, createSignerClient } from "@left-curve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/sdk/actions";
import { KeyAlgo } from "@left-curve/types";
import {
  composeArbitraryTypedData,
  composeTxTypedData,
  getRootDomain,
  hashTypedData,
} from "@left-curve/utils";
import { createConnector } from "./createConnector.js";

import type {
  AccountTypes,
  Address,
  ConnectorId,
  EIP1193Provider,
  Eip712Signature,
  Json,
  Transport,
  TypedDataProperty,
} from "@left-curve/types";

import "@left-curve/types/window";
import type { SignerClient } from "@left-curve/sdk/clients";

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

          const pubKey = await secp256k1RecoverPubKey(ethHashMessage(challenge), signature, true);

          const keyHash = createKeyHash({ pubKey, keyAlgo: KeyAlgo.Secp256k1 });
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
          _client = createSignerClient({
            signer: new ConnectorSigner(this),
            type: "eip1193",
            username: _username,
            transport: _transport,
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

        const credential = { signature: { eip712 } };

        return { credential, keyHash };
      },
      async signTx(signDoc) {
        try {
          const { typedData: types, sender, ...txMessage } = signDoc;
          const provider = await this.getProvider();
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          if (!types) throw new Error("Typed data required");

          const domain = {
            name: getRootDomain(window.location.hostname),
            verifyingContract: sender,
          };

          const typedData = composeTxTypedData(txMessage, domain, types);
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

          const credential = { standard: { signature: { eip712 } } };

          const keyHash = createKeyHash({
            pubKey: await secp256k1RecoverPubKey(hashData, signature, true),
            keyAlgo: KeyAlgo.Secp256k1,
          });

          return { credential, keyHash, signDoc };
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
