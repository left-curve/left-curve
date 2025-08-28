import { persist, subscribeWithSelector } from "zustand/middleware";
import { createStore } from "zustand/vanilla";

import { createPublicClient } from "@left-curve/dango";
import { plainObject, uid } from "@left-curve/dango/utils";

import pkgJson from "../package.json" with { type: "json" };
import { eip6963 } from "./connectors/eip6963.js";
import { type EventData, createEmitter } from "./createEmitter.js";
import { createMipdStore } from "./mipd.js";
import { createStorage } from "./storages/createStorage.js";
import { ConnectionStatus } from "./types/store.js";

import type {
  AccountTypes,
  Address,
  AppConfig,
  Client,
  Denom,
  Flatten,
  Hex,
  PairUpdate,
  PublicClient,
  Transport,
} from "@left-curve/dango/types";

import { invertObject } from "@left-curve/dango/utils";

import { subscriptionsStore } from "./subscriptions.js";
import type { AnyCoin } from "./types/coin.js";
import type { Connector, ConnectorEventMap, CreateConnectorFn } from "./types/connector.js";
import type { EIP6963ProviderDetail } from "./types/eip6963.js";
import type { Config, CreateConfigParameters, State, StoreApi } from "./types/store.js";

export function createConfig<
  transport extends Transport = Transport,
  coin extends AnyCoin = AnyCoin,
>(parameters: CreateConfigParameters<transport, coin>): Config<transport, coin> {
  const {
    multiInjectedProviderDiscovery = true,
    version = 0,
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

  const coins = createStore(() => ({
    byDenom: rest.coins || {},
    bySymbol: Object.values(rest.coins || {}).reduce((acc, coin) => {
      acc[coin.symbol] = coin;
      return acc;
    }, Object.create({})),
  }));

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
        chain: rest.chain,
        transport: rest.transport,
        storage,
        getUsername: () => store.getState().username,
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

  let _appConfig:
    | ({
        addresses: Flatten<AppConfig["addresses"]> & Record<Address, string>;
        accountFactory: { codeHashes: Record<AccountTypes, Hex> };
        pairs: Record<Denom, PairUpdate>;
      } & Omit<AppConfig, "addresses">)
    | undefined;

  async function getAppConfig() {
    if (_appConfig) return _appConfig;
    const client = getClient() as PublicClient;
    const [appConfig, codeHashes, pairs] = await Promise.all([
      client.getAppConfig(),
      client.getAccountTypeCodeHashes(),
      client.getPairs(),
    ]);

    const addresses = plainObject(appConfig.addresses) as Flatten<AppConfig["addresses"]>;

    _appConfig = {
      ...appConfig,
      addresses: {
        ...addresses,
        ...invertObject(addresses),
      },
      accountFactory: { codeHashes },
      pairs: pairs.reduce((acc, pair) => {
        acc[pair.baseDenom] = pair;
        return acc;
      }, Object.create({})),
    };
    return _appConfig;
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
      username: undefined,
      status: ConnectionStatus.Disconnected,
    };
  }

  const currentVersion = Number.parseInt(pkgJson.version);
  const stateCreator = storage
    ? persist(getInitialState, {
        name: "store",
        version,
        storage,
        migrate(state, version) {
          const persistedState = state as State;
          if (version === currentVersion) return persistedState;

          const initialState = getInitialState();
          return { ...initialState };
        },
        partialize(state) {
          const { chainId, connectors, status, current, username } = state;
          return {
            chainId,
            status,
            current,
            username,
            connectors: new Map(
              Array.from(connectors.entries()).map(([key, connection]) => {
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

      if (storage && !store.persist.hasHydrated()) return;
      connectors.setState((x) => [...x, ...newConnectors], true);
      store.setState((x) => ({ ...x, isMipdLoaded: true }));
    });
  }

  const sbStore = subscriptionsStore(getClient() as PublicClient);

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
        current: data.uid,
        username: data.username,
        connectors: new Map(x.connectors).set(data.uid, {
          keyHash: data.keyHash,
          account: data.accounts[0],
          accounts: data.accounts,
          chainId: data.chainId,
          connector: connector,
        }),
        chainId: data.chainId,
        status: ConnectionStatus.Connected,
      };
    });
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
        if (!connector.emitter.listenerCount("connect")) {
          connection.connector.emitter.on("connect", connect);
        }
      }

      x.connectors.delete(data.uid);

      for (const [uid] of x.connectors.entries()) {
        if (uid === data.uid) x.connectors.delete(uid);
      }

      if (x.connectors.size === 0) {
        return {
          ...x,
          username: undefined,
          connectors: new Map(),
          current: null,
          status: ConnectionStatus.Disconnected,
        };
      }

      return {
        ...x,
        connectors: new Map(x.connectors),
      };
    });
  }

  function getCoinInfo(denom: Denom): AnyCoin {
    const allCoins = coins.getState()!;
    if (!denom.includes("dex")) return allCoins.byDenom[denom];
    const [_, __, baseDenom, quoteDenom] = denom.split("/");
    const coinsArray = Object.values(allCoins.byDenom);
    const baseCoin = coinsArray.find((x) => x.denom.includes(baseDenom))!;
    const quoteCoin = coinsArray.find((x) => x.denom.includes(quoteDenom))!;

    return {
      type: "lp",
      symbol: `${baseCoin.symbol}-${quoteCoin.symbol}`,
      denom,
      decimals: 0,
      base: baseCoin,
      quote: quoteCoin,
    };
  }

  return {
    get coins() {
      const state = coins.getState() ?? { byDenom: {}, bySymbol: {} };
      return state;
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
    getCoinInfo,
    getAppConfig,
    getClient,
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
