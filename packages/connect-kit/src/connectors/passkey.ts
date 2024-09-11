import { requestWebAuthnSignature, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8, serialize } from "@leftcurve/encoding";
import { createKeyHash, createUserClient } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { UserClient } from "@leftcurve/sdk/clients";
import { ConnectorSigner } from "@leftcurve/sdk/signers";
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
            rpId: window.location.hostname,
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
          rpId: window.location.hostname,
          userVerification: "preferred",
        });
        return createKeyHash({ credentialId });
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
        const bytes = sha256(serialize(txMessage));

        const { signature, webauthn, credentialId } = await requestWebAuthnSignature({
          challenge: bytes,
          rpId: window.location.hostname,
          userVerification: "preferred",
        });

        const passkeyCredential = encodeUtf8(
          JSON.stringify({
            signature,
            webauthn,
          }),
        );

        const credential = { passkey: encodeBase64(passkeyCredential) };
        const keyHash = createKeyHash({ credentialId });

        return { credential, keyHash, signDoc };
      },
    };
  });
}
