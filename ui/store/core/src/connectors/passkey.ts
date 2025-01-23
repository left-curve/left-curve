import { parseAsn1Signature, requestWebAuthnSignature, sha256 } from "@left-curve/dango/crypto";

import { encodeBase64, encodeUtf8, serialize } from "@left-curve/encoding";

import { createKeyHash, createSignerClient } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";
import { KeyAlgo } from "@left-curve/dango/types";
import { getRootDomain } from "@left-curve/utils";

import { createConnector } from "./createConnector.js";

import type { AccountTypes, SignerClient } from "@left-curve/dango/types";
import type { Address, Transport } from "@left-curve/types";

type PasskeyConnectorParameters = {
  icon?: string;
};

export function passkey(parameters: PasskeyConnectorParameters = {}) {
  let _transport: Transport;
  let _username: string;
  let _client: SignerClient;
  let _isAuthorized = false;

  const { icon } = parameters;

  return createConnector(({ transports, emitter }) => {
    return {
      id: "passkey",
      name: "Passkey",
      type: "passkey",
      icon,
      async connect({ username, chainId, challenge }) {
        _username = username;
        _transport = transports[chainId];

        const client = await this.getClient();

        if (challenge) {
          const { credentialId } = await requestWebAuthnSignature({
            challenge: encodeUtf8(challenge),
            rpId: getRootDomain(window.location.hostname),
            userVerification: "preferred",
          });

          const keyHash = createKeyHash({ credentialId, keyAlgo: KeyAlgo.Secp256r1 });
          const keys = await getKeysByUsername(client, { username });

          if (!Object.keys(keys).includes(keyHash)) throw new Error("Not authorized");
          _isAuthorized = true;
        }

        const accounts = await this.getAccounts();
        emitter.emit("connect", { accounts, chainId, username });
      },
      async disconnect() {
        _isAuthorized = false;
        emitter.emit("disconnect");
      },
      async getClient() {
        if (!_client) {
          _client = createSignerClient({
            signer: this,
            username: _username,
            transport: _transport,
          });
        }
        return _client;
      },
      async getKeyHash() {
        const { credentialId } = await requestWebAuthnSignature({
          challenge: crypto.getRandomValues(new Uint8Array(32)),
          rpId: getRootDomain(window.location.hostname),
          userVerification: "preferred",
        });
        return createKeyHash({ credentialId, keyAlgo: KeyAlgo.Secp256r1 });
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
        const bytes = sha256(serialize(payload));

        const {
          webauthn,
          credentialId,
          signature: asnSignature,
        } = await requestWebAuthnSignature({
          challenge: bytes,
          rpId: getRootDomain(window.location.hostname),
          userVerification: "preferred",
        });

        const signature = parseAsn1Signature(asnSignature);

        const { authenticatorData, clientDataJSON } = webauthn;
        const passkey = {
          sig: encodeBase64(signature),
          client_data: encodeBase64(clientDataJSON),
          authenticator_data: encodeBase64(authenticatorData),
        };

        const keyHash = createKeyHash({ credentialId, keyAlgo: KeyAlgo.Secp256r1 });

        return {
          credential: { standard: { keyHash, signature: { passkey } } },
          payload,
        };
      },
      async signTx(signDoc) {
        const { sender, messages, data, gasLimit } = signDoc;
        const { username, chainId, nonce, expiry } = data;
        const tx = sha256(
          serialize({
            sender,
            gasLimit,
            messages,
            data: { username, chainId, nonce, expiry },
          }),
        );

        const {
          webauthn,
          credentialId,
          signature: asnSignature,
        } = await requestWebAuthnSignature({
          challenge: tx,
          rpId: getRootDomain(window.location.hostname),
          userVerification: "preferred",
        });

        const signature = parseAsn1Signature(asnSignature);

        const { authenticatorData, clientDataJSON } = webauthn;

        const passkey = {
          sig: encodeBase64(signature),
          client_data: encodeBase64(clientDataJSON),
          authenticator_data: encodeBase64(authenticatorData),
        };

        const keyHash = createKeyHash({ credentialId, keyAlgo: KeyAlgo.Secp256r1 });
        const standard = { signature: { passkey }, keyHash };

        return { credential: { standard }, signDoc };
      },
      onConnect({ chainId, username }) {
        _username = username;
        _transport = transports[chainId];
      },
    };
  });
}
