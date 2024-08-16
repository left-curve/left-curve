import type { Account } from "./account";
import type { Chain } from "./chain";
import type { Transport } from "./transports";

/**
 * Client configuration options.
 *
 * @template chain - The type of chain.
 */
export type ClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
> = {
  /** The account associated with the client. */
  account: account;
  /** * Indicates whether to batch requests. */
  batch?: boolean;
  /** The chain to connect to. */
  chain?: Chain | undefined | chain;
  /** The key used for authentication. */
  key?: string | undefined;
  /** The name of the client. */
  name?: string | undefined;
  /** The type of the client. */
  type?: string | undefined;
  /** The RPC transport */
  transport: transport;
};

export type ClientBase<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
> = {
  account: account;
  batch?: boolean | undefined;
  chain: chain;
  key: string;
  name: string;
  query: ReturnType<transport>["query"];
  broadcast: ReturnType<transport>["broadcast"];
  /** The RPC transport */
  transport: ReturnType<transport>["config"];
  type: string;
  uid: string;
};

export type Client<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
  extended extends ClientExtend | undefined = ClientExtend | undefined,
> = ClientBase<transport, chain, account> &
  (extended extends ClientExtend ? extended : unknown) & {
    extend: <const client extends ClientExtend = ClientExtend>(
      fn: (client: Client<transport, chain, account, extended>) => client,
    ) => Client<
      transport,
      chain,
      account,
      client & (extended extends ClientExtend ? extended : unknown)
    >;
  };

export type ClientExtend = { [_ in keyof ClientBase]?: undefined } & {
  [key: string]: unknown;
};
