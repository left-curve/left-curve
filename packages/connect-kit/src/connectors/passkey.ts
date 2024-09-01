import { requestWebAuthnSignature, ripemd160 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, encodeUtf8 } from "@leftcurve/encoding";
import { createBaseClient } from "@leftcurve/sdk";
import { getAccountsByKeyHash, getAccountsByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { Account, Client, Transport } from "@leftcurve/types";

export function passkey() {
  let _transport: Transport;
  let _username: string;
  let _client: Client;
  let _isAuthorized = false;

  return createConnector(({ transports, emitter }) => {
    return {
      id: "passkey",
      name: "Passkey",
      type: "passkey",
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

          const usernames = await getAccountsByKeyHash(_client, { hash: keyHash });
          if (!usernames.includes(username)) throw new Error("Not authorized");
          _isAuthorized = true;
        }
        const accounts = await this.getAccounts();
        emitter.emit("connect", { accounts, chainId });
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
          id: `${_username}/account/${index}`,
          index,
          ...info,
        })) as unknown as Account[];
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
