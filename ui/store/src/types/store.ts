import type {
  AccountTypes,
  Address,
  AppConfig,
  Chain,
  ChainId,
  Client,
  Denom,
  Hex,
  PairUpdate,
  Transport,
  UID,
} from "@left-curve/dango/types";

import type { AnyCoin } from "./coin.js";
import type { Connection, Connector, ConnectorEvents, CreateConnectorFn } from "./connector.js";
import type { MipdStore } from "./mipd.js";
import type { Storage } from "./storage.js";
import type { SubscriptionStore } from "./subscriptions.js";

export const ConnectionStatus = {
  Connected: "connected",
  Connecting: "connecting",
  Disconnected: "disconnected",
  Reconnecting: "reconnecting",
} as const;

export type ConnectionStatusType = (typeof ConnectionStatus)[keyof typeof ConnectionStatus];

export type State = {
  chainId: ChainId;
  isMipdLoaded: boolean;
  current: UID | null;
  username: string | undefined;
  connectors: Map<UID, Connection>;
  status: ConnectionStatusType;
};

export type Config<transport extends Transport = Transport, coin extends AnyCoin = AnyCoin> = {
  readonly chain: Chain;
  readonly coins: Record<Denom, coin>;
  readonly connectors: readonly Connector[];
  readonly storage: Storage;
  readonly state: State;
  readonly subscriptions: SubscriptionStore;
  setState(value: State | ((state: State) => State)): void;
  subscribe<state>(
    selector: (state: State) => state,
    listener: (state: state, previousState: state) => void,
    options?: {
      emitImmediately?: boolean;
      equalityFn?: (a: state, b: state) => boolean;
    },
  ): () => void;
  getAppConfig(): Promise<{
    addresses: AppConfig["addresses"] & Record<Address, string>;
    accountFactory: { codeHashes: Record<AccountTypes, Hex> };
    pairs: Record<Denom, PairUpdate>;
  }>;
  getClient(): Client<transport>;
  _internal: Internal<transport>;
};
export type CreateConfigParameters<
  transport extends Transport = Transport,
  coin extends AnyCoin = AnyCoin,
> = {
  version?: number;
  chain: Chain;
  coins?: Record<Denom, coin>;
  transport: transport;
  ssr?: boolean;
  batch?: boolean;
  storage?: Storage;
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

type Internal<transport extends Transport = Transport> = {
  readonly ssr: boolean;
  readonly mipd: MipdStore | undefined;
  readonly store: StoreApi;
  readonly transport: transport;
  readonly events: ConnectorEvents;
  connectors: {
    setup: (connectorFn: CreateConnectorFn) => Connector;
    setState(value: Connector[] | ((state: Connector[]) => Connector[])): void;
    subscribe(listener: (state: Connector[], prevState: Connector[]) => void): () => void;
  };
};
