import type { Chain, ChainId } from "../../../types/src/chain.js";
import type { Client } from "../../../types/src/client.js";
import type { AnyCoin, Denom } from "../../../types/src/coin.js";
import type {
  Connection,
  Connector,
  ConnectorUId,
  CreateConnectorFn,
} from "../../../types/src/connector.js";
import type { Transport } from "../../../types/src/transports.js";
import type { MipdStore } from "./mipd.js";
import type { Storage } from "./storage.js";

export const ConnectionStatus = {
  Connected: "connected",
  Connecting: "connecting",
  Disconnected: "disconnected",
  Reconnecting: "reconnecting",
} as const;

export type ConnectionStatusType = (typeof ConnectionStatus)[keyof typeof ConnectionStatus];

export type State<chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]]> = {
  chainId: chains[number]["id"];
  isMipdLoaded: boolean;
  connections: Map<ConnectorUId, Connection>;
  connectors: Map<chains[number]["id"], ConnectorUId>;
  status: ConnectionStatusType;
};

export type Config<
  chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
  coin extends AnyCoin = AnyCoin,
> = {
  readonly chains: chains;
  readonly coins: Record<ChainId, Record<Denom, coin>>;
  readonly connectors: readonly Connector[];
  readonly storage: Storage | null;
  readonly state: State<chains>;
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
  _internal: Internal<chains, transports>;
};

export type CreateConfigParameters<
  chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
  coin extends AnyCoin = AnyCoin,
> = {
  chains: chains;
  coins?: Record<ChainId, Record<Denom, coin>>;
  transports: transports;
  ssr?: boolean;
  batch?: boolean;
  storage?: Storage | null;
  multiInjectedProviderDiscovery?: boolean;
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

type Internal<
  chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
> = {
  readonly ssr: boolean;
  readonly mipd: MipdStore | undefined;
  readonly store: StoreApi;
  readonly transports: transports;
  readonly events: ConnectorEvents;
  connectors: {
    setup: (connectorFn: CreateConnectorFn) => Connector;
    setState(value: Connector[] | ((state: Connector[]) => Connector[])): void;
    subscribe(listener: (state: Connector[], prevState: Connector[]) => void): () => void;
  };
};
