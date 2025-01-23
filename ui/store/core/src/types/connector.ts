import type { ChainId, Prettify, Transport, UID } from "@left-curve/types";
import type { Emitter, EventData } from "./emitter.js";

import type { Account, Chain, Signer, SignerClient, Username } from "@left-curve/dango/types";

export type ConnectorId = (typeof ConnectorIds)[keyof typeof ConnectorIds] | (string & {});

export const ConnectorIds = {
  Metamask: "metamask",
  Phantom: "phantom",
  Backpack: "backpack",
  Passkey: "passkey",
} as const;

export type ConnectorType = (typeof ConnectorTypes)[keyof typeof ConnectorTypes];

export const ConnectorTypes = {
  EIP1193: "eip1193",
  Passkey: "passkey",
} as const;

export type Connection = {
  chainId: ChainId;
  account: Account;
  username: Username;
  accounts: readonly Account[];
  connector: Connector;
};

export type Connector = ReturnType<CreateConnectorFn> & {
  emitter: Emitter<ConnectorEventMap>;
  uid: UID;
};

export type ConnectorParameter = {
  connector?: Connector;
};

export type ConnectorEventMap = {
  change: {
    username: Username;
    accounts?: readonly Account[] | undefined;
    chainId?: string;
  };
  connect: {
    username: Username;
    accounts: readonly Account[];
    chainId: string;
  };
  disconnect: never;
  error: {
    error: Error;
  };
  message: {
    type: string;
    data?: unknown | undefined;
  };
};

export type ConnectorEvents = {
  change: (event: EventData<ConnectorEventMap, "change">) => void;
  connect: (event: EventData<ConnectorEventMap, "connect">) => void;
  disconnect: (event: EventData<ConnectorEventMap, "disconnect">) => void;
};

export type CreateConnectorFn<
  provider extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
  transport extends Transport = Transport,
  properties extends Record<string, unknown> = Record<string, unknown>,
> = (config: {
  chains: readonly [Chain, ...Chain[]];
  emitter: Emitter<ConnectorEventMap>;
  transports: Record<string, Transport>;
}) => Prettify<
  properties &
    Signer & {
      readonly id: ConnectorId;
      readonly name: string;
      readonly type: ConnectorType;
      readonly icon?: string;
      readonly rdns?: string;
      setup?(): Promise<void>;
      connect(parameters: {
        username: string;
        chainId: Chain["id"];
        challenge?: string;
      }): Promise<void>;
      disconnect(): Promise<void>;
      getAccounts(): Promise<readonly Account[]>;
      getClient(): Promise<SignerClient<transport>>;
      isAuthorized(): Promise<boolean>;
      switchChain?(parameters: { chainId: string }): Promise<Chain>;
      onAccountsChanged?(accounts: string[]): void;
      onChainChanged?(chainId: string): void;
      onConnect(connectInfo: { chainId: string; username: Username }): void;
      onDisconnect?(error?: Error | undefined): void;
      onMessage?(message: { type: string; data?: unknown }): void;
    } & (provider extends undefined
      ? object
      : {
          getProvider(parameters?: { chainId?: string | undefined } | undefined): Promise<provider>;
        })
>;
