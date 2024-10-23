import { ethHashMessage, secp256k1RecoverPubKey } from "@leftcurve/crypto";
import { decodeHex, encodeBase64, encodeHex, encodeUtf8 } from "@leftcurve/encoding";
import { createKeyHash, createUserClient } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { KeyAlgo } from "@leftcurve/types";
import { composeTypedData, hashTypedData } from "@leftcurve/utils";
import { createConnector } from "./createConnector";

import type {
  AccountTypes,
  Address,
  ConnectorId,
  EIP1193Provider,
  Eip712Credential,
  Transport,
} from "@leftcurve/types";

import "@leftcurve/types/window";
import type { UserClient } from "@leftcurve/sdk/clients";
import { ConnectorSigner } from "@leftcurve/sdk/signers";

type EIP1193ConnectorParameters = {
  id: ConnectorId;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters) {
  let _transport: Transport;
  let _username: string;
  let _client: UserClient;
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
      async requestSignature(signDoc) {
        try {
          const { typedData, ...txMessage } = signDoc;
          const provider = await this.getProvider();
          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          if (!typedData) throw new Error("Typed data required");
          const data = composeTypedData(txMessage, typedData);
          const hashedTypedData = await hashTypedData(data);

          const signature = await provider.request({
            method: "eth_signTypedData_v4",
            params: [controllerAddress, JSON.stringify(data)],
          });

          const eip712: Eip712Credential = {
            sig: encodeBase64(decodeHex(signature.slice(2).substring(0, 128))),
            hash_data: hashedTypedData.toUpperCase().slice(2),
            typed_data: encodeBase64(encodeUtf8(JSON.stringify(data))),
          };

          const credential = { eip712 };

          const keyHash = createKeyHash({
            pubKey: await secp256k1RecoverPubKey(hashedTypedData, signature, true),
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
