import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { encodeBase64, encodeUtf8, serializeJson } from "@left-curve/encoding";

import { session } from "../../../store/src/connectors/session";
import { createStorage } from "../../../store/src/storages/createStorage";
import { createMemoryStorage } from "../../../store/src/storages/memoryStorage";

import type { SigningSession } from "@left-curve/types";

const connectorMocks = vi.hoisted(() => ({
  createSessionSigner: vi.fn(),
  createSignerClient: vi.fn(),
  getAccountStatus: vi.fn(),
  getUser: vi.fn(),
  sessionSignArbitrary: vi.fn(),
  sessionSignTx: vi.fn(),
  toAccount: vi.fn(),
}));

vi.mock("@left-curve/sdk", () => ({
  createSessionSigner: connectorMocks.createSessionSigner,
  createSignerClient: connectorMocks.createSignerClient,
  toAccount: connectorMocks.toAccount,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getUser: connectorMocks.getUser,
}));

const sessionKeyHash = "0x73657373696f6e2d6b65790000000000000000000000000000000000000000";
const userAccountAddress = "0x73657373696f6e2d6163636f756e740000000000";
const secondaryAccountAddress = "0x7365636f6e642d6163636f756e74000000000000";

function createSigningSession(overrides: Partial<SigningSession> = {}): SigningSession {
  return {
    authorization: {
      keyHash: sessionKeyHash,
      signature: {
        eip712: {
          sig: "AQID",
          typed_data: "BAUG",
        },
      },
    },
    keyHash: sessionKeyHash,
    privateKey: new Uint8Array([9, 8, 7]),
    publicKey: new Uint8Array([1, 2, 3]),
    sessionInfo: {
      chainId: "dango-dev-1",
      expireAt: "2000000000",
      sessionKey: "AQID",
    },
    ...overrides,
  };
}

function encodeSessionChallenge(signingSession: SigningSession) {
  return encodeBase64(encodeUtf8(serializeJson(signingSession)));
}

function createSessionStorage() {
  return createStorage<{ session: SigningSession }>({
    key: "test-session",
    storage: createMemoryStorage(),
  });
}

function createSessionConnector({
  getUserIndex = () => 7,
  storage = createSessionStorage(),
  target,
}: {
  getUserIndex?: () => number | undefined;
  storage?: ReturnType<typeof createSessionStorage>;
  target?: Parameters<typeof session>[0]["target"];
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
  const connector = session({
    storage,
    target,
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
    storage,
    transport,
  };
}

describe("session connector", () => {
  beforeEach(() => {
    connectorMocks.createSignerClient.mockReturnValue({
      getAccountStatus: connectorMocks.getAccountStatus,
      uid: "session-client",
    });
    connectorMocks.createSessionSigner.mockReturnValue({
      signArbitrary: connectorMocks.sessionSignArbitrary,
      signTx: connectorMocks.sessionSignTx,
    });
    connectorMocks.getAccountStatus.mockResolvedValue("active");
    connectorMocks.getUser.mockResolvedValue({
      accounts: {
        0: userAccountAddress,
        1: secondaryAccountAddress,
      },
      keys: {
        [sessionKeyHash]: {
          secp256k1: "session-public-key",
        },
      },
      name: "session-user",
    });
    connectorMocks.sessionSignArbitrary.mockResolvedValue({
      credential: {
        session: {
          signature: "arbitrary-session-signature",
        },
      },
      signed: {
        memo: "session message",
      },
    });
    connectorMocks.sessionSignTx.mockResolvedValue({
      credential: {
        session: {
          signature: "tx-session-signature",
        },
      },
      signed: {
        tx: "session tx",
      },
    });
    connectorMocks.toAccount.mockImplementation(({ accountIndex, address, user }) => ({
      accountIndex,
      address,
      username: user.name,
    }));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("decodes a desktop challenge, authorizes the backend key, persists the session, and emits connect", async () => {
    const signingSession = createSigningSession();
    const { chain, connector, emitter, storage, transport } = createSessionConnector();

    await connector.connect({
      chainId: chain.id,
      challenge: encodeSessionChallenge(signingSession),
      userIndex: 7,
    });

    expect(connectorMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      signer: connector,
      transport,
      type: "session",
    });
    expect(connectorMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: connectorMocks.getAccountStatus,
        uid: "session-client",
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(await storage.getItem("session")).toEqual(signingSession);
    expect(connectorMocks.getAccountStatus).toHaveBeenCalledWith({
      address: userAccountAddress,
    });
    expect(emitter.emit).toHaveBeenCalledWith("connect", {
      accounts: [
        {
          accountIndex: 0,
          address: userAccountAddress,
          username: "session-user",
        },
        {
          accountIndex: 1,
          address: secondaryAccountAddress,
          username: "session-user",
        },
      ],
      chainId: chain.id,
      keyHash: sessionKeyHash,
      userIndex: 7,
      userStatus: "active",
      username: "session-user",
    });
  });

  it("connects desktop sessions for backend user index zero", async () => {
    const signingSession = createSigningSession();
    const zeroAccountAddress = "0x7a65726f2d636f6e6e6563742d61636300000000";
    connectorMocks.getUser.mockResolvedValue({
      accounts: {
        0: zeroAccountAddress,
      },
      keys: {
        [sessionKeyHash]: {
          secp256k1: "session-public-key",
        },
      },
      name: "zero-session-user",
    });
    const { chain, connector, emitter, storage } = createSessionConnector();

    await connector.connect({
      chainId: chain.id,
      challenge: encodeSessionChallenge(signingSession),
      userIndex: 0,
    });

    expect(connectorMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: connectorMocks.getAccountStatus,
        uid: "session-client",
      }),
      {
        userIndexOrName: {
          index: 0,
        },
      },
    );
    expect(await storage.getItem("session")).toEqual(signingSession);
    expect(connectorMocks.getAccountStatus).toHaveBeenCalledWith({
      address: zeroAccountAddress,
    });
    expect(emitter.emit).toHaveBeenCalledWith("connect", {
      accounts: [
        {
          accountIndex: 0,
          address: zeroAccountAddress,
          username: "zero-session-user",
        },
      ],
      chainId: chain.id,
      keyHash: sessionKeyHash,
      userIndex: 0,
      userStatus: "active",
      username: "zero-session-user",
    });
  });

  it("rejects missing challenges and unauthorized backend keys before storing or connecting", async () => {
    const signingSession = createSigningSession();
    const first = createSessionConnector();

    await expect(
      first.connector.connect({
        chainId: first.chain.id,
        userIndex: 7,
      }),
    ).rejects.toThrow("challenge is required to recover the session");

    expect(connectorMocks.getUser).not.toHaveBeenCalled();
    expect(await first.storage.getItem("session")).toBeNull();

    connectorMocks.getUser.mockResolvedValueOnce({
      accounts: {
        0: userAccountAddress,
      },
      keys: {},
      name: "session-user",
    });
    const second = createSessionConnector();

    await expect(
      second.connector.connect({
        chainId: second.chain.id,
        challenge: encodeSessionChallenge(signingSession),
        userIndex: 7,
      }),
    ).rejects.toThrow("Not authorized");

    expect(await second.storage.getItem("session")).toBeNull();
    expect(connectorMocks.getAccountStatus).not.toHaveBeenCalled();
    expect(second.emitter.emit).not.toHaveBeenCalled();
  });

  it("reads stored sessions for provider, client, accounts, key hash, and authorization expiry", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-09T00:00:00.000Z"));

    const signingSession = createSigningSession();
    const storage = createSessionStorage();
    storage.setItem("session", signingSession);
    const { chain, connector, transport } = createSessionConnector({ storage });

    await expect(connector.getProvider()).resolves.toEqual(signingSession);
    await expect(connector.getKeyHash()).resolves.toBe(sessionKeyHash);
    await expect(connector.getClient()).resolves.toEqual({
      getAccountStatus: connectorMocks.getAccountStatus,
      uid: "session-client",
    });
    expect(connectorMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      sessionKey: signingSession.sessionInfo.sessionKey,
      signer: connector,
      transport,
      type: "session",
    });
    await expect(connector.getAccounts()).resolves.toEqual([
      {
        accountIndex: 0,
        address: userAccountAddress,
        username: "session-user",
      },
      {
        accountIndex: 1,
        address: secondaryAccountAddress,
        username: "session-user",
      },
    ]);
    await expect(connector.isAuthorized()).resolves.toBe(true);

    storage.setItem(
      "session",
      createSigningSession({
        sessionInfo: {
          ...signingSession.sessionInfo,
          expireAt: "1",
        },
      }),
    );

    await expect(connector.isAuthorized()).resolves.toBe(false);
  });

  it("requires a selected user index for account reads before querying the backend", async () => {
    const storage = createSessionStorage();
    storage.setItem("session", createSigningSession());
    const { connector } = createSessionConnector({
      getUserIndex: () => undefined,
      storage,
    });

    await expect(connector.getAccounts()).rejects.toThrow("session: user index not found");
    expect(connectorMocks.getUser).not.toHaveBeenCalled();
  });

  it("reads stored session accounts when the selected backend user index is zero", async () => {
    const zeroAccountAddress = "0x7a65726f2d73657373696f6e2d6163636f756e74";
    const zeroSecondaryAccountAddress = "0x7a65726f2d7365636f6e64617279000000000000";
    const storage = createSessionStorage();
    storage.setItem("session", createSigningSession());
    connectorMocks.getUser.mockResolvedValue({
      accounts: {
        0: zeroAccountAddress,
        2: zeroSecondaryAccountAddress,
      },
      keys: {
        [sessionKeyHash]: {
          secp256k1: "session-public-key",
        },
      },
      name: "zero-session-user",
    });
    const { connector } = createSessionConnector({
      getUserIndex: () => 0,
      storage,
    });

    await expect(connector.getAccounts()).resolves.toEqual([
      {
        accountIndex: 0,
        address: zeroAccountAddress,
        username: "zero-session-user",
      },
      {
        accountIndex: 2,
        address: zeroSecondaryAccountAddress,
        username: "zero-session-user",
      },
    ]);
    expect(connectorMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: connectorMocks.getAccountStatus,
        uid: "session-client",
      }),
      {
        userIndexOrName: {
          index: 0,
        },
      },
    );
    await expect(connector.isAuthorized()).resolves.toBe(true);
  });

  it("delegates signing to the stored session signer and removes storage on disconnect", async () => {
    const signingSession = createSigningSession();
    const storage = createSessionStorage();
    storage.setItem("session", signingSession);
    const { connector, emitter } = createSessionConnector({ storage });
    const arbitraryPayload = {
      message: {
        memo: "session message",
      },
      primaryType: "Message" as const,
      types: {
        Message: [{ name: "memo", type: "string" }],
      },
    };
    const signDoc = {
      message: {
        data: {
          chainId: "dango-dev-1",
          nonce: 7,
          username: "session-user",
        },
        gas_limit: 500000,
        messages: [],
        sender: userAccountAddress,
      },
    };

    await expect(connector.signArbitrary(arbitraryPayload)).resolves.toEqual({
      credential: {
        session: {
          signature: "arbitrary-session-signature",
        },
      },
      signed: {
        memo: "session message",
      },
    });
    await expect(connector.signTx(signDoc as never)).resolves.toEqual({
      credential: {
        session: {
          signature: "tx-session-signature",
        },
      },
      signed: {
        tx: "session tx",
      },
    });

    expect(connectorMocks.createSessionSigner).toHaveBeenCalledWith(signingSession);
    expect(connectorMocks.sessionSignArbitrary).toHaveBeenCalledWith(arbitraryPayload);
    expect(connectorMocks.sessionSignTx).toHaveBeenCalledWith(signDoc);

    await connector.disconnect();

    expect(await storage.getItem("session")).toBeNull();
    expect(emitter.emit).toHaveBeenCalledWith("disconnect");
  });

  it("can source sessions from a configured target provider after setup", async () => {
    const targetSession = createSigningSession({
      keyHash: "0x7461726765742d73657373696f6e2d6b657900000000000000000000000000",
    });
    const { connector } = createSessionConnector({
      target: {
        id: "desktop-session",
        name: "Desktop Session",
        provider: vi.fn().mockResolvedValue(targetSession),
      },
    });

    await connector.setup?.();

    await expect(connector.getProvider()).resolves.toEqual(targetSession);
    await expect(connector.getKeyHash()).resolves.toBe(targetSession.keyHash);
  });
});
