import {
  createWebAuthnCredential,
  parseAsn1Signature,
  requestWebAuthnSignature,
  sha256,
} from "@left-curve/crypto";

import { encodeBase64, encodeUtf8, serialize } from "@left-curve/encoding";

import { createKeyHash, createSignerClient, toAccount } from "@left-curve/sdk";
import { getUser } from "@left-curve/sdk/actions";
import { getNavigatorOS, getRootDomain } from "@left-curve/utils";

import { createConnector } from "./createConnector.js";

import type { ArbitraryDoc, SignDoc } from "@left-curve/types";
import type { Address } from "@left-curve/types";

type PasskeyConnectorParameters = {
  icon?: string;
};

export function passkey(parameters: PasskeyConnectorParameters = {}) {
  const { icon } = parameters;

  return createConnector<undefined>(({ transport, emitter, getUserIndex, chain }) => {
    return {
      id: "passkey",
      name: "Passkey",
      type: "passkey",
      icon,
      async connect({ userIndex, chainId, challenge, keyHash: _keyHash_ }) {
        const client = createSignerClient({
          signer: this,
          type: "passkey",
          chain,
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

        const user = await getUser(client, { userIndexOrName: { index: userIndex } });

        if (!user.keys[keyHash]) throw new Error("Not authorized");

        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );

        const account = accounts[0];
        const userStatus = await client.getAccountStatus({ address: account.address });

        emitter.emit("connect", {
          accounts,
          chainId,
          userIndex,
          keyHash,
          userStatus,
          username: user.name,
        });
      },
      async disconnect() {
        emitter.emit("disconnect");
      },
      async getClient() {
        return createSignerClient({
          signer: this,
          type: "passkey",
          chain,
          transport,
        });
      },
      async createNewKey(challenge = "Please sign this message to confirm your identity.") {
        const { id, getPublicKey } = await createWebAuthnCredential({
          challenge: encodeUtf8(challenge) as Uint8Array<ArrayBuffer>,
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
        const userIndex = getUserIndex();
        if (userIndex === undefined) throw new Error("passkey: user index not found");
        const user = await getUser(client, { userIndexOrName: { index: userIndex } });
        const accounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
          toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
        );
        return accounts;
      },
      async isAuthorized() {
        const accounts = await this.getAccounts();
        return accounts.length > 0;
      },
      async signArbitrary(payload: ArbitraryDoc) {
        const bytes = sha256(serialize(toArbitraryPayload(payload)));

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
          signed: payload,
        };
      },
      async signTx(signDoc: SignDoc) {
        const tx = sha256(serialize(toSignDocPayload(signDoc)));

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

function toSignDocPayload(signDoc: SignDoc) {
  return {
    sender: signDoc.sender,
    gasLimit: signDoc.gasLimit,
    messages: signDoc.messages,
    data: signDoc.data,
  };
}

function toArbitraryPayload(payload: ArbitraryDoc) {
  if (payload.kind === "session") {
    return {
      chainId: payload.chainId,
      sessionKey: payload.sessionKey,
      expireAt: payload.expireAt,
    };
  }
  return {
    chainId: payload.chainId,
    key: payload.key,
    keyHash: payload.keyHash,
    seed: payload.seed,
    ...(payload.referrer !== undefined ? { referrer: payload.referrer } : {}),
  };
}
