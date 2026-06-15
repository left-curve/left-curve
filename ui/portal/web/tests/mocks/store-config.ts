import { vi } from "vitest";

import { ConnectionStatus } from "../../../../store/src/types/store";

import type { Connector } from "../../../../store/src/types/connector";
import type { Config, State } from "../../../../store/src/types/store";

type SelectorSubscription<T> = {
  equalityFn?: (next: T, previous: T) => boolean;
  listener: (next: T, previous: T) => void;
  previous: T;
  selector: (state: State) => T;
};

function createEmitter() {
  return {
    emit: vi.fn(),
  };
}

function testAddressFromLabel(label: string): `0x${string}` {
  return `0x${Array.from(label)
    .map((char) => char.charCodeAt(0).toString(16).padStart(2, "0"))
    .join("")
    .slice(0, 40)
    .padEnd(40, "0")}`;
}

export function createTestConnector(uid: string, overrides: Partial<Connector> = {}): Connector {
  return {
    connect: vi.fn().mockResolvedValue(undefined),
    disconnect: vi.fn().mockResolvedValue(undefined),
    emitter: createEmitter(),
    getAccounts: vi.fn().mockResolvedValue([]),
    getClient: vi.fn().mockResolvedValue({
      uid: `${uid}-signing-client`,
    }),
    id: uid,
    isAuthorized: vi.fn().mockResolvedValue(true),
    name: uid,
    signArbitrary: vi.fn(),
    signTx: vi.fn(),
    type: "debug",
    uid,
    ...overrides,
  } as unknown as Connector;
}

export function createTestConfig({
  chainId = "dango-dev-1",
  connectors = [],
  current = null,
  status = ConnectionStatus.Disconnected,
  user,
}: {
  chainId?: string;
  connectors?: Connector[];
  current?: string | null;
  status?: State["status"];
  user?: State["user"];
} = {}) {
  const subscriptions = new Set<SelectorSubscription<unknown>>();
  const publicClient = {
    uid: "base-client",
  };
  const extendedPublicClient = {
    uid: "public-client",
  };
  const config = {
    captureError: vi.fn(),
    chain: {
      id: chainId,
    },
    coins: {},
    connectors,
    getClient: vi.fn(() => ({
      ...publicClient,
      extend: vi.fn((actions: unknown) => ({
        ...extendedPublicClient,
        actions,
      })),
    })),
    setState: vi.fn((value: State | ((state: State) => State)) => {
      const previousState = config.state;
      const nextState = typeof value === "function" ? value(previousState) : value;
      config.state = nextState;

      for (const subscription of subscriptions) {
        const nextSelection = subscription.selector(config.state);
        const previousSelection = subscription.previous;
        if (subscription.equalityFn?.(nextSelection, previousSelection)) continue;
        subscription.previous = nextSelection;
        subscription.listener(nextSelection, previousSelection);
      }
    }),
    state: {
      chainId,
      connectors: new Map(
        connectors.map((connector) => [
          connector.uid,
          {
            account: {
              address: testAddressFromLabel(connector.uid),
            },
            accounts: [
              {
                address: testAddressFromLabel(connector.uid),
                index: 0,
                owner: user?.index ?? 7,
              },
            ],
            chainId,
            connector,
            keyHash: `${connector.uid}-key-hash`,
          },
        ]),
      ),
      current,
      isMipdLoaded: true,
      status,
      user,
    },
    storage: {},
    subscribe: vi.fn(
      <T>(
        selector: (state: State) => T,
        listener: (next: T, previous: T) => void,
        options: {
          emitImmediately?: boolean;
          equalityFn?: (next: T, previous: T) => boolean;
        } = {},
      ) => {
        const subscription: SelectorSubscription<T> = {
          equalityFn: options.equalityFn,
          listener,
          previous: selector(config.state),
          selector,
        };
        subscriptions.add(subscription as SelectorSubscription<unknown>);
        if (options.emitImmediately) listener(subscription.previous, subscription.previous);
        return () => {
          subscriptions.delete(subscription as SelectorSubscription<unknown>);
        };
      },
    ),
    subscriptions: {},
    _internal: {
      connectors: {
        setState: vi.fn(),
        setup: vi.fn(),
        subscribe: vi.fn(),
      },
      events: {},
      mipd: undefined,
      ssr: false,
      store: {},
      transport: {},
    },
  } as unknown as Config & {
    setState: ReturnType<typeof vi.fn>;
    getClient: ReturnType<typeof vi.fn>;
  };

  return config;
}
