import type { Account, Username } from "./account.js";
import type { Chain, ChainId } from "./chain.js";
import type { Client } from "./client.js";
import type { Emitter } from "./emitter.js";
import type { JsonValue } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type { SignDoc, Signature, SignedDoc } from "./signature.js";
import type { Signer } from "./signer.js";
import type { Transport } from "./transports.js";

export type ConnectorUId = string;

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
  uid: ConnectorUId;
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
  provider extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
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
  getClient(): Promise<Client<transport, chain, signer, any>>;
  getKeyHash(): Promise<KeyHash>;
  isAuthorized(): Promise<boolean>;
  signArbitrary(data: JsonValue): Promise<{ signature: Signature; keyHash: KeyHash }>;
  signTx(signDoc: signDoc): Promise<SignedDoc>;
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
      });
