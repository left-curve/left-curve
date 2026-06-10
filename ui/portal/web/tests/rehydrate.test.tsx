import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { rehydrate } from "../../../store/src/rehydrate";
import { ConnectionStatus } from "../../../store/src/types/store";

import type { Config, State } from "../../../store/src/types/store";

const rehydrateMocks = vi.hoisted(() => ({
  reconnect: vi.fn(),
}));

vi.mock("../../../store/src/actions/reconnect.js", () => ({
  reconnect: rehydrateMocks.reconnect,
}));

const chain = {
  id: "dango-dev-1",
  name: "Devnet",
};
const accountAddress = "0x7265687964726174652d6163636f756e7400000000";
const keyHash = "0x7265687964726174652d6b65790000000000000000000000000000000000";

function createState(overrides: Partial<State> = {}): State {
  return {
    chainId: "persisted-chain",
    connectors: new Map(),
    current: null,
    isMipdLoaded: true,
    status: ConnectionStatus.Connected,
    user: {
      index: 7,
      status: "active",
      username: "alice",
    },
    ...overrides,
  };
}

function createConnectionMap() {
  return new Map([
    [
      "persisted-connector",
      {
        account: {
          address: accountAddress,
          accountIndex: 0,
          username: "alice",
        },
        accounts: [
          {
            address: accountAddress,
            accountIndex: 0,
            username: "alice",
          },
        ],
        chainId: chain.id,
        connector: {
          id: "wallet",
          name: "Wallet",
          type: "debug",
          uid: "persisted-connector",
        },
        keyHash,
      },
    ],
  ]) as State["connectors"];
}

function createRehydrateConfig({
  hasHydrated = false,
  initialState = createState(),
  isSsr = false,
  mipd,
  storage = {},
}: {
  hasHydrated?: boolean;
  initialState?: State;
  isSsr?: boolean;
  mipd?: Config["_internal"]["mipd"];
  storage?: unknown;
} = {}) {
  let state = initialState;
  const subscriptions = new Set<{
    listener: (next: boolean, previous: boolean) => void;
    previous: boolean;
    selector: (state: State) => boolean;
  }>();
  const setup = vi.fn((connectorFn: unknown) => ({
    connectorFn,
    id: `mipd-${setup.mock.calls.length}`,
    name: "Injected Wallet",
    rdns: "com.injected.wallet",
    type: "eip1193",
    uid: `mipd-${setup.mock.calls.length}`,
  }));
  const connectorsSetState = vi.fn(
    (
      value:
        | Array<Record<string, unknown>>
        | ((connectors: Array<Record<string, unknown>>) => unknown),
    ) =>
      typeof value === "function"
        ? value([
            {
              id: "existing-wallet",
              name: "Existing Wallet",
              rdns: "com.existing.wallet",
              type: "eip1193",
              uid: "existing-wallet",
            },
          ])
        : value,
  );
  const config = {
    chain,
    get state() {
      return state;
    },
    setState: vi.fn((value: State | ((state: State) => State)) => {
      state = typeof value === "function" ? value(state) : value;

      for (const subscription of subscriptions) {
        const nextSelection = subscription.selector(state);
        const previousSelection = subscription.previous;
        subscription.previous = nextSelection;
        subscription.listener(nextSelection, previousSelection);
      }
    }),
    storage,
    subscribe: vi.fn(
      (
        selector: (state: State) => boolean,
        listener: (next: boolean, previous: boolean) => void,
      ) => {
        subscriptions.add({
          listener,
          previous: selector(state),
          selector,
        });
        return () => undefined;
      },
    ),
    _internal: {
      connectors: {
        setState: connectorsSetState,
        setup,
      },
      mipd,
      ssr: isSsr,
      store: {
        persist: {
          hasHydrated: vi.fn(() => hasHydrated),
          rehydrate: vi.fn(),
        },
      },
    },
  } as unknown as Config & {
    setState: ReturnType<typeof vi.fn>;
    subscribe: ReturnType<typeof vi.fn>;
    _internal: Config["_internal"] & {
      connectors: Config["_internal"]["connectors"] & {
        setState: ReturnType<typeof vi.fn>;
        setup: ReturnType<typeof vi.fn>;
      };
      store: Config["_internal"]["store"] & {
        persist: Config["_internal"]["store"]["persist"] & {
          hasHydrated: ReturnType<typeof vi.fn>;
        };
      };
    };
  };

  return {
    config,
    connectorsSetState,
    setup,
  };
}

describe("rehydrate", () => {
  beforeEach(() => {
    rehydrateMocks.reconnect.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("normalizes initial state before mount while preserving reconnectable connections", () => {
    const persistedConnectors = createConnectionMap();
    const initialState = createState({
      chainId: "wrong-chain",
      connectors: persistedConnectors,
      status: ConnectionStatus.Connected,
    });
    const { config } = createRehydrateConfig();

    rehydrate(config, {
      initialState,
      reconnectOnMount: true,
    });

    expect(config._internal.store.persist.hasHydrated).toHaveBeenCalledOnce();
    expect(config.state.chainId).toBe(chain.id);
    expect(config.state.connectors).toBe(persistedConnectors);
    expect(config.state.status).toBe(ConnectionStatus.Reconnecting);
  });

  it("preserves backend user index zero during initial state normalization", () => {
    const persistedConnectors = createConnectionMap();
    const initialState = createState({
      chainId: "wrong-chain",
      connectors: persistedConnectors,
      status: ConnectionStatus.Connected,
      user: {
        index: 0,
        status: "inactive",
        username: "genesis",
      },
    });
    const { config } = createRehydrateConfig();

    rehydrate(config, {
      initialState,
      reconnectOnMount: true,
    });

    expect(config.state.chainId).toBe(chain.id);
    expect(config.state.connectors).toBe(persistedConnectors);
    expect(config.state.status).toBe(ConnectionStatus.Reconnecting);
    expect(config.state.user).toEqual({
      index: 0,
      status: "inactive",
      username: "genesis",
    });
  });

  it("does not overwrite current state with initial state after storage has hydrated", () => {
    const currentState = createState({
      chainId: chain.id,
      connectors: new Map(),
      status: ConnectionStatus.Disconnected,
      user: undefined,
    });
    const { config } = createRehydrateConfig({
      hasHydrated: true,
      initialState: currentState,
    });

    rehydrate(config, {
      initialState: createState({
        connectors: createConnectionMap(),
      }),
      reconnectOnMount: true,
    });

    expect(config.setState).not.toHaveBeenCalled();
    expect(config.state).toBe(currentState);
  });

  it("clears hydrated connections on mount when reconnect is disabled", async () => {
    const { config } = createRehydrateConfig({
      initialState: createState({
        chainId: chain.id,
        connectors: createConnectionMap(),
        status: ConnectionStatus.Connected,
      }),
    });

    const { onMount } = rehydrate(config, {
      reconnectOnMount: false,
    });

    await onMount();

    expect(rehydrateMocks.reconnect).not.toHaveBeenCalled();
    expect(config.state.connectors.size).toBe(0);
    expect(config.state.status).toBe(ConnectionStatus.Disconnected);
  });

  it("reconnects immediately on mount when provider discovery is already loaded", async () => {
    const { config } = createRehydrateConfig({
      initialState: createState({
        chainId: chain.id,
        connectors: createConnectionMap(),
        isMipdLoaded: true,
        status: ConnectionStatus.Reconnecting,
      }),
    });

    const { onMount } = rehydrate(config, {
      reconnectOnMount: true,
    });
    await onMount();

    expect(config.subscribe).not.toHaveBeenCalled();
    expect(rehydrateMocks.reconnect).toHaveBeenCalledOnce();
    expect(rehydrateMocks.reconnect).toHaveBeenCalledWith(config);
  });

  it("waits for MIPD before reconnecting when provider discovery is still loading", async () => {
    const { config } = createRehydrateConfig({
      initialState: createState({
        chainId: chain.id,
        isMipdLoaded: false,
        status: ConnectionStatus.Disconnected,
      }),
    });

    const { onMount } = rehydrate(config, {
      reconnectOnMount: true,
    });
    await onMount();

    expect(config.subscribe).toHaveBeenCalledWith(expect.any(Function), expect.any(Function));
    expect(rehydrateMocks.reconnect).not.toHaveBeenCalled();

    config.setState((state) => ({
      ...state,
      isMipdLoaded: false,
    }));
    expect(rehydrateMocks.reconnect).not.toHaveBeenCalled();

    config.setState((state) => ({
      ...state,
      isMipdLoaded: true,
    }));
    expect(rehydrateMocks.reconnect).toHaveBeenCalledWith(config);
  });

  it("adds SSR MIPD connectors without duplicating existing rdns entries", async () => {
    const injectedProvider = {
      info: {
        icon: "data:image/svg+xml,<svg></svg>",
        name: "Injected Wallet",
        rdns: "com.injected.wallet",
        uuid: "injected-wallet",
      },
      provider: {
        on: vi.fn(),
        removeListener: vi.fn(),
        request: vi.fn(),
      },
    };
    const duplicateProvider = {
      ...injectedProvider,
      info: {
        ...injectedProvider.info,
        rdns: "com.existing.wallet",
        uuid: "existing-wallet",
      },
    };
    const { config, connectorsSetState, setup } = createRehydrateConfig({
      isSsr: true,
      mipd: {
        getProviders: vi.fn(() => [duplicateProvider, injectedProvider]),
      } as unknown as Config["_internal"]["mipd"],
    });

    const { onMount } = rehydrate(config, {
      reconnectOnMount: true,
    });
    await onMount();

    expect(connectorsSetState).toHaveBeenCalledOnce();
    const nextConnectors = connectorsSetState.mock.results[0].value as Array<{
      id: string;
      rdns?: string;
    }>;

    expect(setup).toHaveBeenCalledOnce();
    expect(nextConnectors).toHaveLength(2);
    expect(nextConnectors[0]).toMatchObject({
      id: "existing-wallet",
      rdns: "com.existing.wallet",
    });
    expect(nextConnectors[1]).toMatchObject({
      id: "mipd-1",
      rdns: "com.injected.wallet",
    });
    expect(rehydrateMocks.reconnect).toHaveBeenCalledWith(config);
  });
});
