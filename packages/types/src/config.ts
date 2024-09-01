import type { Chain } from "./chain";
import type { Client } from "./client";
import type { Connection, Connector, CreateConnectorFn } from "./connector";
import type { Storage } from "./storage";
import type { Transport } from "./transports";

export type State<chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]]> = {
  chainId: chains[number]["id"];
  connections: Map<chains[number]["id"], Connection>;
  authorizations: Map<string, chains[number]["id"]>;
  status: "connected" | "connecting" | "disconnected" | "reconnecting";
};

export type Config<
  chains extends readonly [Chain, ...Chain[]] = readonly [Chain, ...Chain[]],
  transports extends Record<chains[number]["id"], Transport> = Record<
    chains[number]["id"],
    Transport
  >,
> = {
  readonly chains: chains;
  readonly connectors: readonly Connector[];
  readonly storage: Storage | null;
  readonly state: State<chains>;
  setState<tchains extends readonly [Chain, ...Chain[]] = chains>(
    value: State<tchains> | ((state: State<tchains>) => State<tchains>),
  ): void;
  subscribe<state>(
    selector: (state: State<chains>) => state,
    listener: (state: state, previousState: state) => void,
    options?:
      | {
          emitImmediately?: boolean | undefined;
        }
      | undefined,
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
  connectors?: CreateConnectorFn[] | undefined;
  storage?: Storage | null | undefined;
  transports: transports;
  batch?: boolean | undefined;
};

export type ConfigParameter<config extends Config = Config> = {
  config?: Config | config | undefined;
};
