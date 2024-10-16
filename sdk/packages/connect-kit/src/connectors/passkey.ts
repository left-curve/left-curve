import { parseAsn1Signature, requestWebAuthnSignature, sha256 } from "@leftcurve/crypto";
import { decodeUtf8, encodeBase64, encodeUtf8, serialize } from "@leftcurve/encoding";
import { createKeyHash, createUserClient } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { UserClient } from "@leftcurve/sdk/clients";
import { ConnectorSigner } from "@leftcurve/sdk/signers";
import { getRootDomain } from "@leftcurve/utils";

import type { AccountTypes, Address, Transport } from "@leftcurve/types";

type PasskeyConnectorParameters = {
  icon?: string;
};

export function passkey(parameters: PasskeyConnectorParameters = {}) {
  let _transport: Transport;
  let _username: string;
  let _client: UserClient;
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

          const keyHash = createKeyHash({ credentialId });
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
          _client = createUserClient({
            transport: _transport,
            signer: new ConnectorSigner(this),
            username: _username,
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
        return createKeyHash({ credentialId });
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
      async requestSignature({ messages, chainId, sequence }) {
        const bytes = sha256(serialize({ messages, chainId, sequence }));

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
        const { origin, crossOrigin } = JSON.parse(decodeUtf8(clientDataJSON));

        const passkey = {
          origin,
          cross_origin: crossOrigin,
          sig: encodeBase64(signature),
          authenticator_data: encodeBase64(authenticatorData),
        };

        const credential = { passkey };
        const keyHash = createKeyHash({ credentialId });

        return { credential, keyHash, signDoc: { messages, chainId, sequence } };
      },
      onConnect({ chainId, username }) {
        _username = username;
        _transport = transports[chainId];
      },
    };
  });
}
