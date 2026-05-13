import { persist, subscribeWithSelector } from "zustand/middleware";
import { createStore } from "zustand/vanilla";

import { createPublicClient } from "@left-curve/dango";
import { uid } from "@left-curve/dango/utils";

import { eip6963 } from "./connectors/eip6963.js";
import { type EventData, createEmitter } from "./createEmitter.js";
import { createMipdStore } from "./mipd.js";
import { createStorage } from "./storages/createStorage.js";
import { ConnectionStatus } from "./types/store.js";

import type { Client, PublicClient, Transport } from "@left-curve/dango/types";

import { subscriptionsStore } from "./subscriptions.js";
import type { Connector, ConnectorEventMap, CreateConnectorFn } from "./types/connector.js";
import type { EIP6963ProviderDetail } from "./types/eip6963.js";
import type { Config, CreateConfigParameters, State, StoreApi } from "./types/store.js";
import { CoinStore } from "./stores/coinStore.js";

export function createConfig<transport extends Transport = Transport>(
  parameters: CreateConfigParameters<transport>,
): Config<transport> {
  const {
    multiInjectedProviderDiscovery = true,
    version = 0,
    storage = createStorage({
      storage:
        typeof window !== "undefined" && window.localStorage ? window.localStorage : undefined,
    }),
    ssr,
    onError,
    ...rest
  } = parameters;

  //////////////////////////////////////////////////////////////////////////////
  // Set up connectors, clients, etc.
  //////////////////////////////////////////////////////////////////////////////

  const mipd =
    typeof window !== "undefined" && multiInjectedProviderDiscovery ? createMipdStore() : undefined;

  const coinsStore = CoinStore.getState();
  coinsStore.setCoins(rest.coins);

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

  function getUserIndex(): number | undefined {
    return store.getState().user?.index;
  }

  function setup(connectorFn: CreateConnectorFn): Connector {
    // Set up emitter with uid and add to connector so they are "linked" together.
    const emitter = createEmitter<ConnectorEventMap>(uid());
    const connector = {
      ...connectorFn({
        emitter,
        chain: rest.chain,
        transport: rest.transport,
        storage,
        getUserIndex,
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

  let _client: Client<transport> | undefined;

  function getClient(): Client<transport> {
    if (_client) return _client;

    const client = createPublicClient({
      chain: rest.chain,
      transport: (parameters) => rest.transport({ ...parameters }),
    });

    _client = client;
    return client;
  }

  //////////////////////////////////////////////////////////////////////////////
  // Create store
  //////////////////////////////////////////////////////////////////////////////

  function getInitialState(): State {
    return {
      isMipdLoaded: !multiInjectedProviderDiscovery,
      chainId: rest.chain.id,
      connectors: new Map(),
      current: null,
      user: undefined,
      status: ConnectionStatus.Disconnected,
    };
  }

  const stateCreator = storage
    ? persist(getInitialState, {
        name: "store",
        version,
        storage,
        migrate(state, savedVersion) {
          if (version === savedVersion) return state as State;

          const persisted = state as Record<string, unknown>;
          const initialState = getInitialState();

          // v1 → v2: migrate { userIndex, userStatus } to { user: { index } }
          if (savedVersion === 1 && persisted) {
            const userIndex = persisted.userIndex as number | undefined;
            return {
              ...initialState,
              current: (persisted.current as State["current"]) ?? null,
              connectors: (persisted.connectors as State["connectors"]) ?? new Map(),
              user: userIndex !== undefined ? { index: userIndex } : undefined,
            } as State;
          }

          return initialState;
        },
        partialize(state) {
          const { chainId, connectors, status, current, user } = state;
          return {
            chainId,
            status,
            current,
            user: user ? { index: user.index } : undefined,
            connectors: new Map(
              Array.from(connectors.entries()).map(([key, connection]) => {
                const { id, name, type, uid } = connection.connector;
                const connector = { id, name, type, uid };
                const { accounts: _, ...rest } = connection;
                return [key, { ...rest, connector }];
              }),
            ),
          };
        },
        merge(persistedState, currentState) {
          if (!persistedState) return currentState;

          if (typeof persistedState === "object" && "status" in persistedState) {
            delete persistedState.status;
          }

          return {
            ...currentState,
            ...persistedState,
            chainId: rest.chain.id,
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

      // The `connectors` vanilla store is not persisted, so its update does
      // not need to wait for hydration. Always apply it — dropping MIPD
      // connectors on the floor during the hydration window used to cause
      // silent wallet-login failures on first page load.
      connectors.setState((x) => [...x, ...newConnectors], true);
      if (!storage || store.persist.hasHydrated()) {
        store.setState((x) => ({ ...x, isMipdLoaded: true }));
      }
    });
  }

  const sbStore = subscriptionsStore(getClient() as PublicClient, { onError });

  //////////////////////////////////////////////////////////////////////////////
  // Emitter listeners
  //////////////////////////////////////////////////////////////////////////////

  function change(data: EventData<ConnectorEventMap, "change">) {
    store.setState((x) => {
      const connection = x.connectors.get(data.uid);
      if (!connection) return x;
      const { chainId, uid } = data;

      return {
        ...x,
        user: {
          index: data.userIndex,
          username: data.username,
          status: data.userStatus,
        },
        connectors: new Map(x.connectors).set(uid, {
          keyHash: data.keyHash,
          accounts: data.accounts ?? connection.accounts,
          account: connection.account,
          connector: connection.connector,
          chainId: chainId ?? connection.chainId,
        }),
      };
    });
  }
  function connect(data: EventData<ConnectorEventMap, "connect">) {
    const connector = connectors.getState().find((c) => c.uid === data.uid);
    if (!connector) {
      // A `connect` event fired for a connector that isn't in the list.
      // This used to silently swallow the event — surface it loudly instead
      // so any remaining race or lifecycle bug shows up in telemetry.
      const error = new Error(`connect event received for unknown connector uid: ${data.uid}`);
      if (onError) onError(error);
      else console.error(error);
      return;
    }

    // Wire ongoing change/disconnect listeners outside the setState reducer so
    // the reducer stays pure. Listeners are idempotent via listenerCount check.
    if (!connector.emitter.listenerCount("change")) {
      connector.emitter.on("change", change);
    }
    if (!connector.emitter.listenerCount("disconnect")) {
      connector.emitter.on("disconnect", disconnect);
    }

    store.setState((x) => ({
      ...x,
      current: data.uid,
      user: {
        index: data.userIndex,
        username: data.username,
        status: data.userStatus,
      },
      connectors: new Map(x.connectors).set(data.uid, {
        keyHash: data.keyHash,
        account: data.accounts[0],
        accounts: data.accounts,
        chainId: data.chainId,
        connector,
      }),
      chainId: data.chainId,
      status: ConnectionStatus.Connected,
    }));
  }
  function disconnect(data: EventData<ConnectorEventMap, "disconnect">) {
    store.setState((x) => {
      const connection = x.connectors.get(data.uid);
      if (connection) {
        const connector = connection.connector;
        if (connector.emitter.listenerCount("change")) {
          connection.connector.emitter.off("change", change);
        }
        if (connector.emitter.listenerCount("disconnect")) {
          connection.connector.emitter.off("disconnect", disconnect);
        }
        // The `connect` listener is attached once in setup() and never
        // removed, so there's no need to re-attach it here.
      }

      x.connectors.delete(data.uid);

      for (const [uid] of x.connectors.entries()) {
        if (uid === data.uid) x.connectors.delete(uid);
      }

      if (x.connectors.size === 0) {
        return {
          ...x,
          connectors: new Map(),
          current: null,
          user: undefined,
          status: ConnectionStatus.Disconnected,
        };
      }

      return {
        ...x,
        connectors: new Map(x.connectors),
      };
    });
  }

  return {
    get coins() {
      return CoinStore.getState();
    },
    get subscriptions() {
      return sbStore;
    },
    get chain() {
      return rest.chain;
    },
    get connectors() {
      return connectors.getState();
    },
    storage,
    getClient,
    captureError(error: unknown) {
      if (onError) onError(error);
      else console.error(error);
    },
    get state() {
      return store.getState();
    },
    setState(value) {
      let newState: State;
      if (typeof value === "function") newState = value(store.getState());
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
      get transport() {
        return rest.transport;
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
