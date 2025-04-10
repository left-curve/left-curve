import {
  createWebAuthnCredential,
  parseAsn1Signature,
  requestWebAuthnSignature,
  sha256,
} from "@left-curve/dango/crypto";

import { encodeBase64, encodeUtf8, serialize } from "@left-curve/dango/encoding";

import { createKeyHash, createSignerClient, toAccount } from "@left-curve/dango";
import { getAccountsByUsername, getKeysByUsername } from "@left-curve/dango/actions";
import { getNavigatorOS, getRootDomain } from "@left-curve/dango/utils";

import { createConnector } from "./createConnector.js";

import type { AccountTypes } from "@left-curve/dango/types";
import type { Address } from "@left-curve/dango/types";

type PasskeyConnectorParameters = {
  icon?: string;
};

export function passkey(parameters: PasskeyConnectorParameters = {}) {
  const { icon } = parameters;

  return createConnector<undefined>(({ transport, emitter, getUsername, chain }) => {
    return {
      id: "passkey",
      name: "Passkey",
      type: "passkey",
      icon,
      async connect({ username, chainId, challenge, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "passkey",
          username,
          transport,
        });

        const keyHash = await (async () => {
          if (_keyHash_) return _keyHash_;
          const c = challenge as string;
          const { credentialId } = await requestWebAuthnSignature({
            challenge: encodeUtf8(c),
            rpId: getRootDomain(window.location.hostname),
            userVerification: "preferred",
          });

          return createKeyHash(credentialId);
        })();

        const keys = await getKeysByUsername(client, { username });

        if (!Object.keys(keys).includes(keyHash)) throw new Error("Not authorized");

        const accountsInfo = await getAccountsByUsername(client, { username });
        const accounts = Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ username, address: address as Address, info: accountInfo }),
        );

        emitter.emit("connect", { accounts, chainId, username, keyHash });
      },
      async disconnect() {
        emitter.emit("disconnect");
      },
      async getClient() {
        const username = getUsername();
        if (!username) throw new Error("passkey: username not found");
        return createSignerClient({
          signer: this,
          type: "passkey",
          username,
          chain,
          transport,
        });
      },
      async createNewKey(challenge = "Please sign this message to confirm your identity.") {
        const { id, getPublicKey } = await createWebAuthnCredential({
          challenge: encodeUtf8(challenge),
          user: {
            name: `${getNavigatorOS()} ${new Date().toLocaleString()}`,
          },
          rp: {
            name: window.document.title,
            id: getRootDomain(window.location.hostname),
          },
          authenticatorSelection: {
            residentKey: "preferred",
            requireResidentKey: false,
            userVerification: "preferred",
          },
        });

        const publicKey = await getPublicKey();
        const key = { secp256r1: encodeBase64(publicKey) };
        const keyHash = createKeyHash(id);

        return { key, keyHash };
      },
      async getKeyHash() {
        const { credentialId } = await requestWebAuthnSignature({
          challenge: crypto.getRandomValues(new Uint8Array(32)),
          rpId: getRootDomain(window.location.hostname),
          userVerification: "preferred",
        });
        return createKeyHash(credentialId);
      },
      async getAccounts() {
        const client = await this.getClient();
        const username = getUsername();
        if (!username) throw new Error("passkey: username not found");
        const accounts = await getAccountsByUsername(client, { username });
        return Object.entries(accounts).map(([address, accountInfo]) => {
          const { index, params } = accountInfo;
          const type = Object.keys(params)[0] as AccountTypes;
          return {
            index,
            params,
            address: address as Address,
            username,
            type: type,
          };
        });
      },
      async isAuthorized() {
        return true;
      },
      async signArbitrary(payload) {
        const { message } = payload;
        const bytes = sha256(serialize(message));

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

        const keyHash = createKeyHash(credentialId);

        return {
          credential: { standard: { keyHash, signature: { passkey } } },
          signed: message,
        };
      },
      async signTx(signDoc) {
        const { message } = signDoc;

        const tx = sha256(serialize(message));

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

        const keyHash = createKeyHash(credentialId);
        const standard = { signature: { passkey }, keyHash };

        return { credential: { standard }, signed: signDoc };
      },
    };
  });
}
