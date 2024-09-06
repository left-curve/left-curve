import type { Chain } from "./chain";
import type { Client } from "./client";
import type { Connection, Connector, ConnectorId, CreateConnectorFn } from "./connector";
import type { Storage } from "./storage";
import type { Transport } from "./transports";

export type State<chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]]> = {
  chainId: chains[number]["id"];
  connections: Map<ConnectorId, Connection>;
  connectors: Map<chains[number]["id"], ConnectorId>;
  status: "connected" | "connecting" | "disconnected" | "reconnecting";
};

export type Config<
  chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
> = {
  readonly ssr: boolean;
  readonly chains: chains;
  readonly connectors: readonly Connector[];
  readonly storage: Storage | null;
  readonly state: State<chains>;
  readonly store: StoreApi;
  setState<tchains extends readonly [Chain, ...Chain[]] = chains>(
    value: State<tchains> | ((state: State<tchains>) => State<tchains>),
  ): void;
  subscribe<state>(
    selector: (state: State<chains>) => state,
    listener: (state: state, previousState: state) => void,
    options?: {
      emitImmediately?: boolean;
      equalityFn?: (a: state, b: state) => boolean;
    },
  ): () => void;

  getClient<chainId extends chains[number]["id"]>(parameters?: {
    chainId?: chainId | chains[number]["id"] | undefined;
  }): Client<transports[chainId], chains[number]>;
};

export type CreateConfigParameters<
  chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
> = {
  chains: chains;
  transports: transports;
  ssr?: boolean;
  batch?: boolean;
  storage?: Storage | null;
  connectors?: CreateConnectorFn[];
};

export type ConfigParameter<config extends Config = Config> = {
  config?: Config | config;
};

export type StoreApi = {
  setState: (partial: State | Partial<State>, replace?: boolean) => void;
  getState: () => State;
  getInitialState: () => State;
  subscribe: (listener: (state: State, prevState: State) => void) => () => void;
  persist: {
    rehydrate: () => Promise<void> | void;
    hasHydrated: () => boolean;
  };
};
