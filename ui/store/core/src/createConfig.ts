import { persist, subscribeWithSelector } from "zustand/middleware";
import { createStore } from "zustand/vanilla";

import { createPublicClient } from "@left-curve/dango";
import { uid } from "@left-curve/dango/utils";

import pkgJson from "../package.json" with { type: "json" };
import { eip6963 } from "./connectors/eip6963.js";
import { type EventData, createEmitter } from "./createEmitter.js";
import { createMipdStore } from "./mipd.js";
import { createStorage } from "./storages/createStorage.js";
import { ConnectionStatus } from "./types/store.js";

import type { AnyCoin, Chain, Client, Transport } from "@left-curve/dango/types";

import type { Connector, ConnectorEventMap, CreateConnectorFn } from "./types/connector.js";
import type { EIP6963ProviderDetail } from "./types/eip6963.js";
import type { Config, CreateConfigParameters, State, StoreApi } from "./types/store.js";

export function createConfig<
  const chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
  coin extends AnyCoin = AnyCoin,
>(parameters: CreateConfigParameters<chains, transports, coin>): Config<chains, transports, coin> {
  const {
    multiInjectedProviderDiscovery = true,
    storage = createStorage({
      storage:
        typeof window !== "undefined" && window.localStorage ? window.localStorage : undefined,
    }),
    ssr,
    ...rest
  } = parameters;

  //////////////////////////////////////////////////////////////////////////////
  // Set up connectors, clients, etc.
  //////////////////////////////////////////////////////////////////////////////

  const mipd =
    typeof window !== "undefined" && multiInjectedProviderDiscovery ? createMipdStore() : undefined;

  const chains = createStore(() => rest.chains);
  const coins = createStore(() => rest.coins);
  const connectors = createStore(() => {
    const collection = [];
    const rdnsSet = new Set<string>();
    for (const connectorFn of rest.connectors ?? []) {
      const connector = setup(connectorFn);
      collection.push(connector);
      if (!ssr && connector.rdns) rdnsSet.add(connector.rdns);
    }
    if (!ssr && mipd) {
      const providers = mipd.getProviders();
      for (const provider of providers) {
        if (rdnsSet.has(provider.info.rdns)) continue;
        collection.push(setup(eip6963(provider as EIP6963ProviderDetail)));
      }
    }
    return collection;
  });

  function setup(connectorFn: CreateConnectorFn): Connector {
    // Set up emitter with uid and add to connector so they are "linked" together.
    const emitter = createEmitter<ConnectorEventMap>(uid());
    const connector = {
      ...connectorFn({
        emitter,
        chains: chains.getState(),
        transports: rest.transports,
      }),
      emitter,
      uid: emitter.uid,
    };

    // Start listening for `connect` events on connector setup
    // This allows connectors to "connect" themselves without user interaction
    // (e.g. MetaMask's "Manually connect to current site")
    emitter.on("connect", connect);
    connector.setup?.();

    return connector;
  }

  const clients = new Map<string, Client<Transport, chains[number]>>();

  function getClient(
    config: { chainId?: string | undefined } = {},
  ): Client<Transport, chains[number]> {
    const chainId = config.chainId ?? store.getState().chainId;

    if (!chainId) throw new Error("Chain id not provided");

    {
      const client = clients.get(chainId);
      if (client) return client;
    }

    const chain = chains.getState().find((x) => x.id === chainId);

    // chainId specified and not configured
    if (config.chainId && !chain) throw new Error("Chain not configured");

    {
      const client = clients.get(store.getState().chainId);
      if (client) return client;
    }

    if (!chain) throw new Error("Chain not configured");

    {
      const chainId = chain.id as chains[number]["id"];

      const client = createPublicClient({
        chain,
        transport: (parameters) => rest.transports[chainId]({ ...parameters }),
      });

      clients.set(chainId, client);
      return client;
    }
  }

  function validatePersistedChainId(persistedState: unknown, defaultChainId: string) {
    return persistedState &&
      typeof persistedState === "object" &&
      "chainId" in persistedState &&
      typeof persistedState.chainId === "string" &&
      chains.getState().some((x) => x.id === persistedState.chainId)
      ? persistedState.chainId
      : defaultChainId;
  }

  //////////////////////////////////////////////////////////////////////////////
  // Create store
  //////////////////////////////////////////////////////////////////////////////

  function getInitialState(): State {
    return {
      isMipdLoaded: !multiInjectedProviderDiscovery,
      chainId: chains.getState()[0].id,
      connections: new Map(),
      connectors: new Map(),
      status: ConnectionStatus.Disconnected,
    };
  }

  const currentVersion = Number.parseInt(pkgJson.version);
  const stateCreator = storage
    ? persist(getInitialState, {
        name: "store",
        version: 0.2,
        storage,
        migrate(state, version) {
          const persistedState = state as State;
          if (version === currentVersion) return persistedState;

          const initialState = getInitialState();
          const chainId = validatePersistedChainId(persistedState, initialState.chainId);
          return { ...initialState, chainId };
        },
        partialize(state) {
          const { chainId, connections, connectors, status } = state;
          return {
            chainId,
            status,
            connectors,
            connections: new Map(
              Array.from(connections.entries()).map(([key, connection]) => {
                const { id, name, type, uid } = connection.connector;
                const connector = { id, name, type, uid };
                return [key, { ...connection, connector }];
              }),
            ),
          };
        },
        merge(persistedState, currentState) {
          if (!persistedState) return currentState;

          if (typeof persistedState === "object" && "status" in persistedState) {
            delete persistedState.status;
          }

          const chainId = validatePersistedChainId(persistedState, currentState.chainId);

          return {
            ...currentState,
            ...persistedState,
            chainId,
          };
        },
      })
    : getInitialState;

  const store = createStore(subscribeWithSelector(stateCreator));

  if (multiInjectedProviderDiscovery) {
    const timeout = setTimeout(() => store.setState((x) => ({ ...x, isMipdLoaded: true })), 500);
    // EIP-6963 subscribe for new wallet providers
    mipd?.subscribe((providerDetails) => {
      clearTimeout(timeout);
      const connectorIdSet = new Set();
      const connectorRdnsSet = new Set();
      for (const connector of connectors.getState()) {
        connectorIdSet.add(connector.id);
        if (connector.rdns) connectorRdnsSet.add(connector.rdns);
      }

      const newConnectors: Connector[] = [];
      for (const providerDetail of providerDetails) {
        if (connectorRdnsSet.has(providerDetail.info.rdns)) continue;
        const connector = setup(eip6963(providerDetail as EIP6963ProviderDetail));
        if (connectorIdSet.has(connector.id)) continue;
        newConnectors.push(connector);
      }

      if (storage && !store.persist.hasHydrated()) return;
      connectors.setState((x) => [...x, ...newConnectors], true);
      store.setState((x) => ({ ...x, isMipdLoaded: true }));
    });
  }

  //////////////////////////////////////////////////////////////////////////////
  // Emitter listeners
  //////////////////////////////////////////////////////////////////////////////

  function change(data: EventData<ConnectorEventMap, "change">) {
    store.setState((x) => {
      const connection = x.connections.get(data.uid);
      if (!connection) return x;
      const { chainId, uid } = data;
      if (chainId) x.connectors.set(chainId, uid);

      return {
        ...x,
        connections: new Map(x.connections).set(uid, {
          keyHash: data.keyHash,
          accounts: data.accounts ?? connection.accounts,
          account: connection.account,
          connector: connection.connector,
          username: data.username,
          chainId: chainId ?? connection.chainId,
        }),
      };
    });
  }
  function connect(data: EventData<ConnectorEventMap, "connect">) {
    store.setState((x) => {
      const connector = connectors.getState().find((x) => x.uid === data.uid);
      if (!connector) return x;

      if (connector.emitter.listenerCount("connect")) {
        connector.emitter.off("connect", change);
      }

      if (!connector.emitter.listenerCount("change")) {
        connector.emitter.on("change", change);
      }
      if (!connector.emitter.listenerCount("disconnect")) {
        connector.emitter.on("disconnect", disconnect);
      }

      return {
        ...x,
        connections: new Map(x.connections).set(data.uid, {
          keyHash: data.keyHash,
          account: data.accounts[0],
          accounts: data.accounts,
          chainId: data.chainId,
          username: data.username,
          connector: connector,
        }),
        chainId: data.chainId,
        connectors: new Map(x.connectors).set(data.chainId, data.uid),
        status: ConnectionStatus.Connected,
      };
    });
  }
  function disconnect(data: EventData<ConnectorEventMap, "disconnect">) {
    store.setState((x) => {
      const connection = x.connections.get(data.uid);
      if (connection) {
        const connector = connection.connector;
        if (connector.emitter.listenerCount("change")) {
          connection.connector.emitter.off("change", change);
        }
        if (connector.emitter.listenerCount("disconnect")) {
          connection.connector.emitter.off("disconnect", disconnect);
        }
        if (!connector.emitter.listenerCount("connect")) {
          connection.connector.emitter.on("connect", connect);
        }
      }

      x.connections.delete(data.uid);

      for (const [chainId, uid] of x.connectors.entries()) {
        if (uid === data.uid) x.connectors.delete(chainId);
      }

      if (x.connections.size === 0) {
        return {
          ...x,
          connections: new Map(),
          chainIdToConnection: new Map(),
          status: ConnectionStatus.Disconnected,
        };
      }

      return {
        ...x,
        connections: new Map(x.connections),
      };
    });
  }

  return {
    get coins() {
      return coins.getState() ?? {};
    },
    get chains() {
      return chains.getState();
    },
    get connectors() {
      return connectors.getState();
    },
    storage,
    getClient,
    get state() {
      return store.getState() as unknown as State<chains>;
    },
    setState(value) {
      let newState: State;
      if (typeof value === "function") newState = value(store.getState() as any);
      else newState = value;

      // Reset state if it got set to something not matching the base state
      const initialState = getInitialState();
      if (typeof newState !== "object") newState = initialState;
      const isCorrupt = Object.keys(initialState).some((x) => !(x in newState));
      if (isCorrupt) newState = initialState;

      store.setState(newState, true);
    },
    subscribe(selector, listener, options) {
      return store.subscribe(
        selector as unknown as (state: State) => any,
        listener,
        options
          ? {
              ...options,
              fireImmediately: options.emitImmediately,
            }
          : undefined,
      );
    },
    _internal: {
      ssr: ssr ?? false,
      get mipd() {
        return mipd;
      },
      get store() {
        return store as StoreApi;
      },
      get transports() {
        return rest.transports;
      },
      get events() {
        return { change, connect, disconnect };
      },
      connectors: {
        setup,
        setState(value) {
          return connectors.setState(
            typeof value === "function" ? value(connectors.getState()) : value,
            true,
          );
        },
        subscribe(listener) {
          return connectors.subscribe(listener);
        },
      },
    },
  };
}
