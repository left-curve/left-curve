import type { Chain } from "./chain";
import type { Signer } from "./signer";
import type { Transport } from "./transports";

/**
 * Client configuration options.
 *
 */
export type ClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
> = {
  /** The signer used for sign the txs. */
  signer?: signer;
  /** Indicates whether to batch requests. */
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
  signer extends Signer | undefined = undefined,
> = {
  signer: signer;
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
  signer extends Signer | undefined = undefined,
  extended extends ClientExtend | undefined = ClientExtend | undefined,
> = ClientBase<transport, chain, signer> &
  (extended extends ClientExtend ? extended : unknown) & {
    extend: <const client extends ClientExtend = ClientExtend>(
      fn: (client: Client<transport, chain, signer, extended>) => client,
    ) => Client<
      transport,
      chain,
      signer,
      client & (extended extends ClientExtend ? extended : unknown)
    >;
  };

export type ClientExtend = { [_ in keyof ClientBase]?: undefined } & {
  [key: string]: unknown;
};
