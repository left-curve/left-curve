import { requestWebAuthnSignature } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { createBaseClient, createKeyHash } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { Address, Client, Transport } from "@leftcurve/types";

type PasskeyConnectorParameters = {
  icon?: string;
};

export function passkey(parameters: PasskeyConnectorParameters = {}) {
  let _transport: Transport;
  let _username: string;
  let _client: Client;
  let _isAuthorized = false;

  const { icon } = parameters;

  return createConnector<undefined, { bytes: Uint8Array }>(({ transports, emitter }) => {
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
        if (!_client) _client = createBaseClient({ transport: _transport });
        return _client;
      },
      async getAccounts() {
        const client = await this.getClient();
        const accounts = await getAccountsByUsername(client, { username: _username });
        return Object.entries(accounts).map(([address, type]) => ({
          address: address as Address,
          username: _username,
          type,
        }));
      },
      async isAuthorized() {
        return _isAuthorized;
      },
      async requestSignature({ bytes }) {
        const { signature, webauthn } = await requestWebAuthnSignature({
          challenge: bytes,
          rpId: window.location.hostname,
          userVerification: "preferred",
        });

        const credential = encodeUtf8(
          JSON.stringify({
            signature,
            webauthn,
          }),
        );

        return { passkey: encodeBase64(credential) };
      },
    };
  });
}
