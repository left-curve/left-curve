import type { Account } from "./account";
import type { Chain } from "./chain";
import type { Client } from "./client";
import type { Credential } from "./credential";
import type { Emitter } from "./emitter";
import type { Storage } from "./storage";
import type { Transport } from "./transports";

export type Connection = {
  chainId: string;
  accounts: readonly Account[];
  connector: Connector;
};

export type Connector = ReturnType<CreateConnectorFn> & {
  emitter: Emitter<ConnectorEventMap>;
  uid: string;
};

export type ConnectorEventMap = {
  change: {
    accounts?: readonly Account[] | undefined;
    chainId?: string;
  };
  connect: { accounts: readonly Account[]; chainId: string };
  disconnect: never;
  error: { error: Error };
  message: { type: string; data?: unknown | undefined };
};

export type CreateConnectorFn<
  provider = unknown,
  properties extends Record<string, unknown> = Record<string, unknown>,
  storageItem extends Record<string, unknown> = Record<string, unknown>,
> = (config: {
  chains: readonly [Chain, ...Chain[]];
  emitter: Emitter<ConnectorEventMap>;
  transports: Record<string, Transport>;
  storage?: Storage<storageItem> | null | undefined;
}) => {
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
  requestSignature(parameters: { bytes: Uint8Array }): Promise<Credential>;
  getProvider?(parameters?: { chainId?: string | undefined } | undefined): Promise<provider>;
  switchChain?(parameters: { chainId: string }): Promise<Chain>;

  onAccountsChanged?(accounts: string[]): void;
  onChainChanged?(chainId: string): void;
  onConnect?(connectInfo: { chainId: string }): void;
  onDisconnect?(error?: Error | undefined): void;
  onMessage?(message: { type: string; data?: unknown }): void;
} & properties;
