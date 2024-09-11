import type { Account, Username } from "./account";
import type { Chain, ChainId } from "./chain";
import type { Client } from "./client";
import type { Emitter } from "./emitter";
import type { KeyHash } from "./key";
import type { SignDoc, SignedDoc } from "./signature";
import type { Signer } from "./signer";
import type { Transport } from "./transports";

export type ConnectorId = string;

export type ConnectorType = (typeof ConnectorType)[keyof typeof ConnectorType];

export const ConnectorType = {
  EIP1193: "eip1193",
  Passkey: "passkey",
} as const;

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
  provider = undefined,
  chain extends Chain = Chain,
  signer extends Signer = Signer,
  signDoc extends SignDoc = SignDoc,
  transport extends Transport = Transport,
  properties extends Record<string, unknown> = Record<string, unknown>,
> = (config: {
  chains: readonly [Chain, ...Chain[]];
  emitter: Emitter<ConnectorEventMap>;
  transports: Record<string, Transport>;
}) => properties & {
  readonly id: string;
  readonly icon?: string | undefined;
  readonly name: string;
  readonly type: ConnectorType;
  setup?(): Promise<void>;
  connect(parameters: {
    username: string;
    chainId: Chain["id"];
    challenge?: string;
  }): Promise<void>;
  disconnect(): Promise<void>;
  getAccounts(): Promise<readonly Account[]>;
  getClient(): Promise<Client<transport, chain, signer>>;
  getKeyHash(): Promise<KeyHash>;
  isAuthorized(): Promise<boolean>;
  requestSignature(signDoc: signDoc): Promise<SignedDoc>;
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
