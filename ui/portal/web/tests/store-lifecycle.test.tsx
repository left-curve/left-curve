import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createConfig } from "../../../store/src/createConfig";
import { reconnect } from "../../../store/src/actions/reconnect";
import { createStorage } from "../../../store/src/storages/createStorage";
import { createMemoryStorage } from "../../../store/src/storages/memoryStorage";
import { ConnectionStatus } from "../../../store/src/types/store";

import type { Connector, CreateConnectorFn } from "../../../store/src/types/connector";
import type { Connection, State } from "../../../store/src/types/store";

const lifecycleMocks = vi.hoisted(() => ({
  createPublicClient: vi.fn(() => ({
    uid: "public-client",
  })),
  getAccountStatus: vi.fn(),
  getUser: vi.fn(),
  toAccount: vi.fn(({ accountIndex, address, user }) => ({
    accountIndex,
    address,
    username: user.name,
  })),
}));

vi.mock("@left-curve/sdk", () => ({
  createPublicClient: lifecycleMocks.createPublicClient,
  toAccount: lifecycleMocks.toAccount,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getAccountStatus: lifecycleMocks.getAccountStatus,
  getUser: lifecycleMocks.getUser,
}));

const chain = {
  id: "dango-dev-1",
  name: "Devnet",
};
const transport = {
  type: "http",
};
const primaryAddress = "0x73746f72652d6163636f756e742d300000000000";
const secondaryAddress = "0x73746f72652d6163636f756e742d310000000000";
const activeKeyHash = "0x73746f72652d6c6966656379636c652d6b65790000000000000000000000";
const rejectedKeyHash = "0x72656a65637465642d6b6579000000000000000000000000000000000000";

function createTestStorage() {
  return createStorage({
    key: `store-lifecycle-${Math.random()}`,
    storage: createMemoryStorage(),
  });
}

function createLifecycleConnector({
  id,
  isAuthorized = true,
}: {
  id: string;
  isAuthorized?: boolean;
}) {
  const methods = {
    connect: vi.fn(),
    disconnect: vi.fn(),
    getAccounts: vi.fn().mockResolvedValue([]),
    getClient: vi.fn().mockResolvedValue({ uid: `${id}-client` }),
    isAuthorized: vi.fn().mockResolvedValue(isAuthorized),
    signArbitrary: vi.fn(),
    signTx: vi.fn(),
  };

  const connectorFn = ((context) => {
    methods.connect.mockImplementation(
      async ({ chainId, userIndex }: { chainId: string; userIndex: number }) => {
        context.emitter.emit("connect", {
          accounts: [
            {
              accountIndex: 0,
              address: primaryAddress,
              username: "alice",
            },
            {
              accountIndex: 1,
              address: secondaryAddress,
              username: "alice",
            },
          ],
          chainId,
          keyHash: activeKeyHash,
          userIndex,
          userStatus: "active",
          username: "alice",
        });
      },
    );
    methods.disconnect.mockImplementation(async () => {
      context.emitter.emit("disconnect");
    });

    return {
      id,
      name: id,
      type: "debug",
      ...methods,
    };
  }) as CreateConnectorFn;

  return {
    connectorFn,
    methods,
  };
}

function persistedConnectorSnapshot(connector: Connector, uid: string): Connector {
  return {
    id: connector.id,
    name: connector.name,
    type: connector.type,
    uid,
  } as Connector;
}

function persistedConnection({
  accountAddress,
  chainId = chain.id,
  connector,
  keyHash,
  uid,
}: {
  accountAddress: string;
  chainId?: string;
  connector: Connector;
  keyHash: string;
  uid: string;
}): Connection {
  return {
    account: {
      address: accountAddress,
      accountIndex: 0,
      username: "persisted-user",
    },
    accounts: [
      {
        address: accountAddress,
        accountIndex: 0,
        username: "persisted-user",
      },
    ],
    chainId,
    connector: persistedConnectorSnapshot(connector, uid),
    keyHash,
  } as Connection;
}

describe("store lifecycle", () => {
  beforeEach(() => {
    lifecycleMocks.getAccountStatus.mockResolvedValue("active");
    lifecycleMocks.getUser.mockResolvedValue({
      accounts: {
        0: primaryAddress,
        1: secondaryAddress,
      },
      keys: {
        [activeKeyHash]: {
          secp256k1: "public-key",
        },
      },
      name: "backend-user",
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("updates config state from connector connect, change, and disconnect events", async () => {
    const lifecycleConnector = createLifecycleConnector({ id: "debug-wallet" });
    const config = createConfig({
      chain,
      coins: {},
      connectors: [lifecycleConnector.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage: createTestStorage(),
      transport,
    });
    const connector = config.connectors[0];

    expect(config.state).toMatchObject({
      chainId: chain.id,
      current: null,
      isMipdLoaded: true,
      status: ConnectionStatus.Disconnected,
      user: undefined,
    });

    await connector.connect({
      chainId: chain.id,
      challenge: "signin",
      userIndex: 7,
    });

    expect(config.state.status).toBe(ConnectionStatus.Connected);
    expect(config.state.current).toBe(connector.uid);
    expect(config.state.user).toEqual({
      index: 7,
      status: "active",
      username: "alice",
    });
    expect(config.state.connectors.get(connector.uid)).toMatchObject({
      account: {
        address: primaryAddress,
      },
      accounts: [
        {
          address: primaryAddress,
        },
        {
          address: secondaryAddress,
        },
      ],
      chainId: chain.id,
      keyHash: activeKeyHash,
    });
    expect(connector.emitter.listenerCount("change")).toBe(1);
    expect(connector.emitter.listenerCount("disconnect")).toBe(1);

    connector.emitter.emit("change", {
      accounts: [
        {
          accountIndex: 1,
          address: secondaryAddress,
          username: "alice-updated",
        },
      ],
      chainId: "dango-dev-2",
      keyHash: rejectedKeyHash,
      userIndex: 8,
      userStatus: "active",
      username: "alice-updated",
    });

    expect(config.state.user).toEqual({
      index: 8,
      status: "active",
      username: "alice-updated",
    });
    expect(config.state.connectors.get(connector.uid)).toMatchObject({
      accounts: [
        {
          address: secondaryAddress,
        },
      ],
      chainId: "dango-dev-2",
      keyHash: rejectedKeyHash,
    });

    await connector.disconnect();

    expect(config.state.current).toBeNull();
    expect(config.state.user).toBeUndefined();
    expect(config.state.status).toBe(ConnectionStatus.Disconnected);
    expect(config.state.connectors.size).toBe(0);
    expect(connector.emitter.listenerCount("change")).toBe(0);
    expect(connector.emitter.listenerCount("disconnect")).toBe(0);
  });

  it("persists only reconnectable connection snapshots after login", async () => {
    const lifecycleConnector = createLifecycleConnector({ id: "persisted-wallet" });
    const storage = createTestStorage();
    const config = createConfig({
      chain,
      coins: {},
      connectors: [lifecycleConnector.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage,
      transport,
    });
    const connector = config.connectors[0];

    await connector.connect({
      chainId: chain.id,
      challenge: "signin",
      userIndex: 7,
    });

    const persisted = storage.getItem("store") as { state: State; version: number };
    const persistedConnection = persisted.state.connectors.get(connector.uid);

    expect(persisted.version).toBe(0);
    expect(persisted.state).toMatchObject({
      chainId: chain.id,
      current: connector.uid,
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
      },
    });
    expect(persisted.state.user).not.toHaveProperty("username");
    expect(persisted.state.user).not.toHaveProperty("status");
    expect(persisted.state.connectors.size).toBe(1);
    expect(persistedConnection).toMatchObject({
      account: {
        address: primaryAddress,
      },
      chainId: chain.id,
      connector: {
        id: "persisted-wallet",
        name: "persisted-wallet",
        type: "debug",
        uid: connector.uid,
      },
      keyHash: activeKeyHash,
    });
    expect(persistedConnection).not.toHaveProperty("accounts");
    expect(persistedConnection?.connector).not.toHaveProperty("connect");
    expect(persistedConnection?.connector).not.toHaveProperty("emitter");
    expect(persistedConnection?.connector).not.toHaveProperty("getClient");
  });

  it("persists backend user index zero from connector connect events", async () => {
    const lifecycleConnector = createLifecycleConnector({ id: "zero-wallet" });
    const storage = createTestStorage();
    const config = createConfig({
      chain,
      coins: {},
      connectors: [lifecycleConnector.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage,
      transport,
    });
    const connector = config.connectors[0];

    await connector.connect({
      chainId: chain.id,
      challenge: "signin-zero",
      userIndex: 0,
    });

    expect(config.state.user).toEqual({
      index: 0,
      status: "active",
      username: "alice",
    });

    const persisted = storage.getItem("store") as { state: State; version: number };

    expect(persisted.state).toMatchObject({
      current: connector.uid,
      status: ConnectionStatus.Connected,
      user: {
        index: 0,
      },
    });
    expect(persisted.state.user).not.toHaveProperty("username");
    expect(persisted.state.user).not.toHaveProperty("status");
    expect(persisted.state.connectors.get(connector.uid)).toMatchObject({
      account: {
        address: primaryAddress,
      },
      keyHash: activeKeyHash,
    });
  });

  it("updates repeated connect events without duplicating connector listeners", async () => {
    const lifecycleConnector = createLifecycleConnector({ id: "repeat-wallet" });
    const config = createConfig({
      chain,
      coins: {},
      connectors: [lifecycleConnector.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage: createTestStorage(),
      transport,
    });
    const connector = config.connectors[0];

    await connector.connect({
      chainId: chain.id,
      challenge: "signin",
      userIndex: 7,
    });
    await connector.connect({
      chainId: chain.id,
      challenge: "signin-again",
      userIndex: 8,
    });

    expect(lifecycleConnector.methods.connect).toHaveBeenCalledTimes(2);
    expect(connector.emitter.listenerCount("change")).toBe(1);
    expect(connector.emitter.listenerCount("disconnect")).toBe(1);
    expect(config.state).toMatchObject({
      chainId: chain.id,
      current: connector.uid,
      status: ConnectionStatus.Connected,
      user: {
        index: 8,
        status: "active",
        username: "alice",
      },
    });
    expect([...config.state.connectors.keys()]).toEqual([connector.uid]);

    connector.emitter.emit("change", {
      chainId: "dango-dev-2",
      keyHash: rejectedKeyHash,
      userIndex: 9,
      userStatus: "inactive",
      username: "alice-updated",
    });

    expect(config.state.user).toEqual({
      index: 9,
      status: "inactive",
      username: "alice-updated",
    });
    expect(config.state.connectors.get(connector.uid)).toMatchObject({
      chainId: "dango-dev-2",
      keyHash: rejectedKeyHash,
    });
  });

  it("preserves the active connection when another connected wallet disconnects", async () => {
    const first = createLifecycleConnector({ id: "first-wallet" });
    const second = createLifecycleConnector({ id: "second-wallet" });
    const config = createConfig({
      chain,
      coins: {},
      connectors: [first.connectorFn, second.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage: createTestStorage(),
      transport,
    });
    const firstConnector = config.connectors[0];
    const secondConnector = config.connectors[1];

    await firstConnector.connect({
      chainId: chain.id,
      challenge: "signin-first",
      userIndex: 7,
    });
    await secondConnector.connect({
      chainId: chain.id,
      challenge: "signin-second",
      userIndex: 8,
    });

    expect(config.state.current).toBe(secondConnector.uid);
    expect([...config.state.connectors.keys()]).toEqual([firstConnector.uid, secondConnector.uid]);

    await firstConnector.disconnect();

    expect(first.methods.disconnect).toHaveBeenCalledOnce();
    expect(second.methods.disconnect).not.toHaveBeenCalled();
    expect(firstConnector.emitter.listenerCount("change")).toBe(0);
    expect(firstConnector.emitter.listenerCount("disconnect")).toBe(0);
    expect(secondConnector.emitter.listenerCount("change")).toBe(1);
    expect(secondConnector.emitter.listenerCount("disconnect")).toBe(1);
    expect(config.state).toMatchObject({
      current: secondConnector.uid,
      status: ConnectionStatus.Connected,
      user: {
        index: 8,
        status: "active",
        username: "alice",
      },
    });
    expect([...config.state.connectors.keys()]).toEqual([secondConnector.uid]);
    expect(config.state.connectors.get(secondConnector.uid)?.connector).toBe(secondConnector);
  });

  it("migrates legacy persisted connection state to the current reconnect shape", () => {
    const lifecycleConnector = createLifecycleConnector({ id: "legacy-wallet" });
    const storage = createTestStorage();
    const legacyConnectorUid = "legacy-wallet-uid";
    const legacyConnectors = new Map([
      [
        legacyConnectorUid,
        {
          account: {
            accountIndex: 1,
            address: secondaryAddress,
            username: "legacy-user",
          },
          accounts: [
            {
              accountIndex: 1,
              address: secondaryAddress,
              username: "legacy-user",
            },
          ],
          chainId: "legacy-chain",
          connector: {
            id: "legacy-wallet",
            name: "Legacy Wallet",
            type: "debug",
            uid: legacyConnectorUid,
          },
          keyHash: activeKeyHash,
        },
      ],
    ]) as State["connectors"];

    storage.setItem("store", {
      state: {
        chainId: "legacy-chain",
        connectors: legacyConnectors,
        current: legacyConnectorUid,
        status: ConnectionStatus.Connected,
        userIndex: 7,
      },
      version: 1,
    });

    const config = createConfig({
      chain,
      coins: {},
      connectors: [lifecycleConnector.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage,
      transport,
      version: 2,
    });

    expect(config.state).toMatchObject({
      chainId: chain.id,
      current: legacyConnectorUid,
      status: ConnectionStatus.Disconnected,
      user: {
        index: 7,
      },
    });
    expect(config.state.user).not.toHaveProperty("username");
    expect(config.state.user).not.toHaveProperty("status");
    expect(config.state.connectors).toEqual(legacyConnectors);
    expect(config.state.connectors.get(legacyConnectorUid)).toMatchObject({
      account: {
        address: secondaryAddress,
      },
      chainId: "legacy-chain",
      connector: {
        id: "legacy-wallet",
        uid: legacyConnectorUid,
      },
      keyHash: activeKeyHash,
    });
  });

  it("reports unknown connector connect events without mutating connection state", () => {
    const onError = vi.fn();
    const config = createConfig({
      chain,
      coins: {},
      connectors: [],
      multiInjectedProviderDiscovery: false,
      onError,
      storage: createTestStorage(),
      transport,
    });

    config._internal.events.connect({
      accounts: [
        {
          accountIndex: 0,
          address: primaryAddress,
          username: "alice",
        },
      ],
      chainId: chain.id,
      keyHash: activeKeyHash,
      uid: "missing-connector",
      userIndex: 7,
      userStatus: "active",
      username: "alice",
    });

    expect(onError).toHaveBeenCalledWith(
      new Error("connect event received for unknown connector uid: missing-connector"),
    );
    expect(config.state).toMatchObject({
      current: null,
      status: ConnectionStatus.Disconnected,
      user: undefined,
    });
    expect(config.state.connectors.size).toBe(0);
  });

  it("reconnects persisted connections against current connector instances and backend account data", async () => {
    const authorized = createLifecycleConnector({ id: "authorized-wallet" });
    const unauthorized = createLifecycleConnector({
      id: "unauthorized-wallet",
      isAuthorized: false,
    });
    const config = createConfig({
      chain,
      coins: {},
      connectors: [authorized.connectorFn, unauthorized.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage: createTestStorage(),
      transport,
    });
    const authorizedConnector = config.connectors[0];
    const unauthorizedConnector = config.connectors[1];
    const persistedAuthorizedUid = "persisted-authorized-wallet";
    const persistedUnauthorizedUid = "persisted-unauthorized-wallet";

    config.setState({
      ...config.state,
      connectors: new Map([
        [
          persistedAuthorizedUid,
          persistedConnection({
            accountAddress: secondaryAddress,
            connector: authorizedConnector,
            keyHash: activeKeyHash,
            uid: persistedAuthorizedUid,
          }),
        ],
        [
          persistedUnauthorizedUid,
          persistedConnection({
            accountAddress: primaryAddress,
            connector: unauthorizedConnector,
            keyHash: rejectedKeyHash,
            uid: persistedUnauthorizedUid,
          }),
        ],
      ]),
      current: persistedAuthorizedUid,
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: undefined,
        username: "persisted-user",
      },
    } as State);

    await reconnect(config);

    expect(lifecycleMocks.getUser).toHaveBeenCalledWith(
      {
        uid: "public-client",
      },
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(lifecycleMocks.getAccountStatus).toHaveBeenCalledWith(
      {
        uid: "public-client",
      },
      {
        address: primaryAddress,
      },
    );
    expect(authorized.methods.isAuthorized).toHaveBeenCalledOnce();
    expect(unauthorized.methods.isAuthorized).toHaveBeenCalledOnce();
    expect(config.state.status).toBe(ConnectionStatus.Connected);
    expect(config.state.current).toBe(authorizedConnector.uid);
    expect(config.state.user).toEqual({
      index: 7,
      status: "active",
      username: "backend-user",
    });
    expect([...config.state.connectors.keys()]).toEqual([authorizedConnector.uid]);
    expect(config.state.connectors.get(authorizedConnector.uid)).toMatchObject({
      account: {
        address: secondaryAddress,
      },
      accounts: [
        {
          address: primaryAddress,
          username: "backend-user",
        },
        {
          address: secondaryAddress,
          username: "backend-user",
        },
      ],
      chainId: chain.id,
      keyHash: activeKeyHash,
    });
    expect(config.state.connectors.get(authorizedConnector.uid)?.connector).toBe(
      authorizedConnector,
    );
    expect(authorizedConnector.emitter.listenerCount("change")).toBe(1);
    expect(authorizedConnector.emitter.listenerCount("disconnect")).toBe(1);
  });

  it("ignores persisted reconnect snapshots from a different backend chain", async () => {
    const authorized = createLifecycleConnector({ id: "authorized-wallet" });
    const staleChain = createLifecycleConnector({ id: "stale-chain-wallet" });
    const config = createConfig({
      chain,
      coins: {},
      connectors: [authorized.connectorFn, staleChain.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage: createTestStorage(),
      transport,
    });
    const authorizedConnector = config.connectors[0];
    const staleChainConnector = config.connectors[1];
    const persistedAuthorizedUid = "persisted-authorized-wallet";
    const persistedStaleChainUid = "persisted-stale-chain-wallet";

    config.setState({
      ...config.state,
      connectors: new Map([
        [
          persistedAuthorizedUid,
          persistedConnection({
            accountAddress: primaryAddress,
            connector: authorizedConnector,
            keyHash: activeKeyHash,
            uid: persistedAuthorizedUid,
          }),
        ],
        [
          persistedStaleChainUid,
          persistedConnection({
            accountAddress: secondaryAddress,
            chainId: "dango-dev-2",
            connector: staleChainConnector,
            keyHash: rejectedKeyHash,
            uid: persistedStaleChainUid,
          }),
        ],
      ]),
      current: persistedAuthorizedUid,
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: undefined,
        username: "persisted-user",
      },
    } as State);

    await reconnect(config);

    expect(authorized.methods.isAuthorized).toHaveBeenCalledOnce();
    expect(staleChain.methods.isAuthorized).not.toHaveBeenCalled();
    expect(config.state.status).toBe(ConnectionStatus.Connected);
    expect(config.state.current).toBe(authorizedConnector.uid);
    expect([...config.state.connectors.keys()]).toEqual([authorizedConnector.uid]);
    expect(config.state.connectors.get(authorizedConnector.uid)).toMatchObject({
      account: {
        address: primaryAddress,
      },
      chainId: chain.id,
      keyHash: activeKeyHash,
    });
  });

  it("falls back to the first backend account when a persisted reconnect account is stale", async () => {
    const authorized = createLifecycleConnector({ id: "authorized-wallet" });
    const config = createConfig({
      chain,
      coins: {},
      connectors: [authorized.connectorFn],
      multiInjectedProviderDiscovery: false,
      storage: createTestStorage(),
      transport,
    });
    const authorizedConnector = config.connectors[0];
    const persistedAuthorizedUid = "persisted-authorized-wallet";
    const staleAddress = "0x7374616c652d6163636f756e7400000000000000";

    config.setState({
      ...config.state,
      connectors: new Map([
        [
          persistedAuthorizedUid,
          persistedConnection({
            accountAddress: staleAddress,
            connector: authorizedConnector,
            keyHash: activeKeyHash,
            uid: persistedAuthorizedUid,
          }),
        ],
      ]),
      current: persistedAuthorizedUid,
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: undefined,
        username: "persisted-user",
      },
    } as State);

    await reconnect(config);

    expect(lifecycleMocks.getUser).toHaveBeenCalledWith(
      {
        uid: "public-client",
      },
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(lifecycleMocks.getAccountStatus).toHaveBeenCalledWith(
      {
        uid: "public-client",
      },
      {
        address: primaryAddress,
      },
    );
    expect(config.state.status).toBe(ConnectionStatus.Connected);
    expect(config.state.current).toBe(authorizedConnector.uid);
    expect(config.state.connectors.get(authorizedConnector.uid)).toMatchObject({
      account: {
        address: primaryAddress,
      },
      accounts: [
        {
          address: primaryAddress,
          username: "backend-user",
        },
        {
          address: secondaryAddress,
          username: "backend-user",
        },
      ],
      chainId: chain.id,
      keyHash: activeKeyHash,
    });
  });
});
