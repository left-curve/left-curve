import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { encodeUtf8, serialize } from "@left-curve/encoding";

import { passkey } from "../../../store/src/connectors/passkey";

const connectorMocks = vi.hoisted(() => ({
  createKeyHash: vi.fn((value: string | Uint8Array) => {
    if (value === "credential-id") {
      return "0x706173736b65792d6b65790000000000000000000000000000000000000000";
    }
    if (value === "created-passkey-id") {
      return "0x637265617465642d706173736b657900000000000000000000000000000000";
    }
    return "0x66616c6c6261636b2d706173736b65790000000000000000000000000000";
  }),
  createSignerClient: vi.fn(),
  createWebAuthnCredential: vi.fn(),
  getAccountStatus: vi.fn(),
  getPublicKey: vi.fn(),
  getUser: vi.fn(),
  parseAsn1Signature: vi.fn(),
  requestWebAuthnSignature: vi.fn(),
  sha256: vi.fn(),
  toAccount: vi.fn(),
}));

vi.mock("@left-curve/crypto", () => ({
  createWebAuthnCredential: connectorMocks.createWebAuthnCredential,
  parseAsn1Signature: connectorMocks.parseAsn1Signature,
  requestWebAuthnSignature: connectorMocks.requestWebAuthnSignature,
  sha256: connectorMocks.sha256,
}));

vi.mock("@left-curve/sdk", () => ({
  createKeyHash: connectorMocks.createKeyHash,
  createSignerClient: connectorMocks.createSignerClient,
  toAccount: connectorMocks.toAccount,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getUser: connectorMocks.getUser,
}));

const passkeyCredentialId = "credential-id";
const passkeyKeyHash = "0x706173736b65792d6b65790000000000000000000000000000000000000000";
const createdPasskeyHash = "0x637265617465642d706173736b657900000000000000000000000000000000";
const userAccountAddress = "0x706173736b65792d6163636f756e740000000000";
const secondaryAccountAddress = "0x7365636f6e642d706173736b6579000000000000";
const signatureDigest = new Uint8Array([30, 31, 32]);
const asnSignature = new Uint8Array([9, 9, 9]);
const parsedSignature = new Uint8Array([1, 2, 3]);
const authenticatorData = new Uint8Array([4, 5, 6]);
const clientDataJSON = new Uint8Array([7, 8, 9]);

function createPasskeyConnector({
  getUserIndex = () => 7,
}: {
  getUserIndex?: () => number | undefined;
} = {}) {
  const emitter = {
    emit: vi.fn(),
  };
  const chain = {
    id: "dango-dev-1",
    name: "Devnet",
  };
  const transport = {
    type: "http",
  };
  const connector = passkey({
    icon: "/passkey.svg",
  })({
    chain,
    emitter,
    getUserIndex,
    transport,
  } as never);

  return {
    chain,
    connector,
    emitter,
    transport,
  };
}

describe("passkey connector", () => {
  beforeEach(() => {
    window.document.title = "Dango";
    connectorMocks.createSignerClient.mockReturnValue({
      getAccountStatus: connectorMocks.getAccountStatus,
      uid: "passkey-client",
    });
    connectorMocks.createWebAuthnCredential.mockResolvedValue({
      getPublicKey: connectorMocks.getPublicKey,
      id: "created-passkey-id",
    });
    connectorMocks.getAccountStatus.mockResolvedValue("active");
    connectorMocks.getPublicKey.mockResolvedValue(new Uint8Array([10, 11, 12]));
    connectorMocks.getUser.mockResolvedValue({
      accounts: {
        0: userAccountAddress,
        1: secondaryAccountAddress,
      },
      keys: {
        [passkeyKeyHash]: {
          secp256r1: "passkey-public-key",
        },
      },
      name: "passkey-user",
    });
    connectorMocks.parseAsn1Signature.mockReturnValue(parsedSignature);
    connectorMocks.requestWebAuthnSignature.mockResolvedValue({
      credentialId: passkeyCredentialId,
      signature: asnSignature,
      webauthn: {
        authenticatorData,
        clientDataJSON,
      },
    });
    connectorMocks.sha256.mockReturnValue(signatureDigest);
    connectorMocks.toAccount.mockImplementation(({ accountIndex, address, user }) => ({
      accountIndex,
      address,
      username: user.name,
    }));
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("authenticates with WebAuthn, authorizes the backend key, and emits account status", async () => {
    const { chain, connector, emitter, transport } = createPasskeyConnector();

    await connector.connect({
      chainId: chain.id,
      challenge: "signin-passkey",
      userIndex: 7,
    });

    expect(connectorMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      signer: connector,
      transport,
      type: "passkey",
    });
    expect(connectorMocks.requestWebAuthnSignature).toHaveBeenCalledWith({
      challenge: encodeUtf8("signin-passkey"),
      rpId: "localhost",
      userVerification: "preferred",
    });
    expect(connectorMocks.createKeyHash).toHaveBeenCalledWith(passkeyCredentialId);
    expect(connectorMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: connectorMocks.getAccountStatus,
        uid: "passkey-client",
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(connectorMocks.getAccountStatus).toHaveBeenCalledWith({
      address: userAccountAddress,
    });
    expect(emitter.emit).toHaveBeenCalledWith("connect", {
      accounts: [
        {
          accountIndex: 0,
          address: userAccountAddress,
          username: "passkey-user",
        },
        {
          accountIndex: 1,
          address: secondaryAccountAddress,
          username: "passkey-user",
        },
      ],
      chainId: chain.id,
      keyHash: passkeyKeyHash,
      userIndex: 7,
      userStatus: "active",
      username: "passkey-user",
    });
  });

  it("creates passkey keys with the expected WebAuthn relying party and encoded public key", async () => {
    const { connector } = createPasskeyConnector();

    await expect(connector.createNewKey("create-passkey")).resolves.toEqual({
      key: {
        secp256r1: "CgsM",
      },
      keyHash: createdPasskeyHash,
    });

    expect(connectorMocks.createWebAuthnCredential).toHaveBeenCalledWith({
      authenticatorSelection: {
        requireResidentKey: false,
        residentKey: "preferred",
        userVerification: "preferred",
      },
      challenge: encodeUtf8("create-passkey"),
      rp: {
        id: "localhost",
        name: "Dango",
      },
      user: {
        name: expect.any(String),
      },
    });
    expect(connectorMocks.getPublicKey).toHaveBeenCalledOnce();
    expect(connectorMocks.createKeyHash).toHaveBeenCalledWith("created-passkey-id");
  });

  it("rejects unauthorized backend keys before account status or connection emission", async () => {
    connectorMocks.getUser.mockResolvedValueOnce({
      accounts: {
        0: userAccountAddress,
      },
      keys: {},
      name: "passkey-user",
    });
    const { chain, connector, emitter } = createPasskeyConnector();

    await expect(
      connector.connect({
        chainId: chain.id,
        keyHash: passkeyKeyHash,
        userIndex: 7,
      }),
    ).rejects.toThrow("Not authorized");

    expect(connectorMocks.requestWebAuthnSignature).not.toHaveBeenCalled();
    expect(connectorMocks.getAccountStatus).not.toHaveBeenCalled();
    expect(emitter.emit).not.toHaveBeenCalled();
  });

  it("disconnects passkey sessions through the connector event without WebAuthn or backend calls", async () => {
    const { connector, emitter } = createPasskeyConnector();

    await connector.disconnect();

    expect(emitter.emit).toHaveBeenCalledWith("disconnect");
    expect(connectorMocks.requestWebAuthnSignature).not.toHaveBeenCalled();
    expect(connectorMocks.createSignerClient).not.toHaveBeenCalled();
    expect(connectorMocks.getUser).not.toHaveBeenCalled();
  });

  it("reads key hashes and account authorization through deterministic WebAuthn and backend calls", async () => {
    const { connector } = createPasskeyConnector();

    await expect(connector.getKeyHash()).resolves.toBe(passkeyKeyHash);
    expect(connectorMocks.requestWebAuthnSignature).toHaveBeenCalledWith({
      challenge: expect.any(Uint8Array),
      rpId: "localhost",
      userVerification: "preferred",
    });
    expect(connectorMocks.requestWebAuthnSignature.mock.calls[0][0].challenge).toHaveLength(32);

    await expect(connector.getAccounts()).resolves.toEqual([
      {
        accountIndex: 0,
        address: userAccountAddress,
        username: "passkey-user",
      },
      {
        accountIndex: 1,
        address: secondaryAccountAddress,
        username: "passkey-user",
      },
    ]);
    await expect(connector.isAuthorized()).resolves.toBe(true);
  });

  it("requires a selected user index for account reads before querying the backend", async () => {
    const { connector } = createPasskeyConnector({
      getUserIndex: () => undefined,
    });

    await expect(connector.getAccounts()).rejects.toThrow("passkey: user index not found");
    expect(connectorMocks.getUser).not.toHaveBeenCalled();
  });

  it("signs arbitrary messages and transactions as backend passkey credentials", async () => {
    const { connector } = createPasskeyConnector();
    const arbitraryPayload = {
      message: {
        accountAddress: userAccountAddress,
        memo: "confirm passkey session",
      },
      primaryType: "Message" as const,
      types: {
        Message: [
          { name: "account_address", type: "address" },
          { name: "memo", type: "string" },
        ],
      },
    };
    const signDoc = {
      domain: {
        chainId: 1,
        name: "Dango",
        verifyingContract: "0x0000000000000000000000000000000000000000",
      },
      message: {
        data: {
          chainId: "dango-dev-1",
          nonce: 7,
          username: "passkey-user",
        },
        gas_limit: 500000,
        messages: [
          {
            transfer: {
              to: userAccountAddress,
            },
          },
        ],
        sender: userAccountAddress,
      },
      primaryType: "Message" as const,
      types: {
        EIP712Domain: [
          { name: "name", type: "string" },
          { name: "chainId", type: "uint256" },
          { name: "verifyingContract", type: "address" },
        ],
        Message: [
          { name: "sender", type: "address" },
          { name: "data", type: "Metadata" },
          { name: "gas_limit", type: "uint32" },
          { name: "messages", type: "TxMessage[]" },
        ],
        Metadata: [
          { name: "username", type: "string" },
          { name: "chainId", type: "string" },
          { name: "nonce", type: "uint32" },
        ],
        Transfer: [{ name: "to", type: "address" }],
        TxMessage: [{ name: "transfer", type: "Transfer" }],
      },
    };

    await expect(connector.signArbitrary(arbitraryPayload)).resolves.toEqual({
      credential: {
        standard: {
          keyHash: passkeyKeyHash,
          signature: {
            passkey: {
              authenticator_data: "BAUG",
              client_data: "BwgJ",
              sig: "AQID",
            },
          },
        },
      },
      signed: arbitraryPayload.message,
    });
    expect(connectorMocks.sha256).toHaveBeenNthCalledWith(1, serialize(arbitraryPayload.message));
    expect(connectorMocks.requestWebAuthnSignature).toHaveBeenNthCalledWith(1, {
      challenge: signatureDigest,
      rpId: "localhost",
      userVerification: "preferred",
    });
    expect(connectorMocks.parseAsn1Signature).toHaveBeenNthCalledWith(1, asnSignature);

    await expect(connector.signTx(signDoc)).resolves.toEqual({
      credential: {
        standard: {
          keyHash: passkeyKeyHash,
          signature: {
            passkey: {
              authenticator_data: "BAUG",
              client_data: "BwgJ",
              sig: "AQID",
            },
          },
        },
      },
      signed: signDoc,
    });
    expect(connectorMocks.sha256).toHaveBeenNthCalledWith(2, serialize(signDoc.message));
    expect(connectorMocks.requestWebAuthnSignature).toHaveBeenNthCalledWith(2, {
      challenge: signatureDigest,
      rpId: "localhost",
      userVerification: "preferred",
    });
    expect(connectorMocks.parseAsn1Signature).toHaveBeenNthCalledWith(2, asnSignature);
  });
});
