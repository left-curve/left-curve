import type {
  Chain,
  Client,
  Config,
  Connector,
  ConnectorEventMap,
  CreateConfigParameters,
  CreateConnectorFn,
  EventData,
  State,
  StoreApi,
  Transport,
} from "@leftcurve/types";

import { createEmitter } from "./createEmitter";
import { createStorage } from "./storages/createStorage";

import { createBaseClient } from "@leftcurve/sdk";
import { uid } from "@leftcurve/utils";
import { persist, subscribeWithSelector } from "zustand/middleware";
import { createStore } from "zustand/vanilla";

export function createConfig<
  const chains extends readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport>,
>(parameters: CreateConfigParameters<chains, transports>): Config<chains, transports> {
  const {
    storage = createStorage({
      storage:
        typeof window !== "undefined" && window.localStorage ? window.localStorage : undefined,
    }),
    ...rest
  } = parameters;

  //////////////////////////////////////////////////////////////////////////////
  // Set up connectors, clients, etc.
  //////////////////////////////////////////////////////////////////////////////

  const chains = createStore(() => rest.chains);
  const connectors = createStore(() => [...(rest.connectors ?? [])].map(setup));

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

      const client = createBaseClient<Transport, chains[number]>({
        chain,
        batch: rest.batch ?? false,
        transport: (parameters) => rest.transports[chainId]({ ...parameters }),
      });

      clients.set(chainId, client);
      return client;
    }
  }

  //////////////////////////////////////////////////////////////////////////////
  // Create store
  //////////////////////////////////////////////////////////////////////////////

  function getInitialState(): State {
    return {
      chainId: chains.getState()[0].id,
      connections: new Map(),
      connectors: new Map(),
      status: "disconnected",
    };
  }

  const stateCreator = storage
    ? persist(getInitialState, {
        version: 0,
        name: "store",
        storage,
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

          return {
            ...currentState,
            ...persistedState,
          };
        },
      })
    : getInitialState;

  const store = createStore(subscribeWithSelector(stateCreator));

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
          accounts: data.accounts ?? connection.accounts,
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
          accounts: data.accounts,
          chainId: data.chainId,
          username: data.username,
          connector: connector,
        }),
        chainId: data.chainId,
        connectors: new Map(x.connectors).set(data.chainId, data.uid),
        status: "connected",
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
          status: "disconnected",
        };
      }

      return {
        ...x,
        connections: new Map(x.connections),
      };
    });
  }

  return {
    ssr: rest.ssr ?? false,
    get store() {
      return store as StoreApi;
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
  };
}
