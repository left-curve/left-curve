import { requestWebAuthnSignature, ripemd160 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, encodeUtf8 } from "@leftcurve/encoding";
import { createBaseClient } from "@leftcurve/sdk";
import { getAccountsByUsername, getKeysByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { Client, Transport } from "@leftcurve/types";

type PasskeyConnectorParameters = {
  icon?: string;
};

export function passkey(parameters: PasskeyConnectorParameters) {
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
        await this.getClient();
        if (challenge) {
          const { credentialId } = await requestWebAuthnSignature({
            challenge: encodeUtf8(challenge),
            rpId: window.location.hostname,
            userVerification: "preferred",
          });
          const keyHash = encodeHex(ripemd160(encodeUtf8(credentialId))).toUpperCase();
          const keys = await getKeysByUsername(_client, { username });
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
