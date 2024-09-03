import type { Account, Username } from "./account";
import type { Chain, ChainId } from "./chain";
import type { Client } from "./client";
import type { Credential } from "./credential";
import type { Emitter } from "./emitter";
import type { Storage } from "./storage";
import type { Transport } from "./transports";

export type ConnectorId = string;

export type Connection = {
  chainId: ChainId;
  username: Username;
  accounts: readonly Account[];
  connector: Connector;
};

export type Connector = ReturnType<CreateConnectorFn> & {
  emitter: Emitter<ConnectorEventMap>;
  uid: ConnectorId;
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

export type CreateConnectorFn<
  provider = unknown,
  signDoc = unknown,
  properties extends Record<string, unknown> = Record<string, unknown>,
  storageItem extends Record<string, unknown> = Record<string, unknown>,
> = (config: {
  chains: readonly [Chain, ...Chain[]];
  emitter: Emitter<ConnectorEventMap>;
  transports: Record<string, Transport>;
  storage?: Storage<storageItem> | null | undefined;
}) => properties & {
  readonly id: string;
  readonly icon?: string | undefined;
  readonly name: string;
  readonly type: string;
  setup?(): Promise<void>;
  connect(parameters: {
    username: string;
    chainId: Chain["id"];
    challenge?: string;
  }): Promise<void>;
  disconnect(): Promise<void>;
  getAccounts(): Promise<readonly Account[]>;
  getClient(): Promise<Client>;
  isAuthorized(): Promise<boolean>;
  requestSignature(signDoc: signDoc): Promise<Credential>;
  switchChain?(parameters: { chainId: string }): Promise<Chain>;
  onAccountsChanged?(accounts: string[]): void;
  onChainChanged?(chainId: string): void;
  onConnect?(connectInfo: { chainId: string }): void;
  onDisconnect?(error?: Error | undefined): void;
  onMessage?(message: { type: string; data?: unknown }): void;
} & (provider extends undefined
    ? object
    : {
        getProvider(parameters?: { chainId?: string | undefined } | undefined): Promise<provider>;
      });
