import type { Username } from "./account.js";
import type { Chain } from "./chain.js";
import type { Signer } from "./signer.js";
import type { Transport } from "./transports.js";

/**
 * Client configuration options.
 */
export type ClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
> = {
  /** The signer used for sign the txs. */
  signer?: signer;
  /** The username of the signer. */
  username?: Username;
  /** The chain to connect to. */
  chain?: chain;
  /** The name of the client. */
  name?: string;
  /** The type of the client. */
  type?: string;
  /** The RPC transport */
  transport: transport;
};

export type Client<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
  extended extends ClientExtend | undefined = ClientExtend | undefined,
> = {
  signer: signer;
  username?: Username;
  chain?: chain;
  name: string;
  request: ReturnType<transport>["request"];
  transport: ReturnType<transport>["config"];
  type: string;
  uid: string;
} & (extended extends ClientExtend ? extended : unknown) & {
    extend: <const client extends ClientExtend = ClientExtend>(
      fn: (client: Client<transport, chain, signer, extended>) => client,
    ) => Client<
      transport,
      chain,
      signer,
      client & (extended extends ClientExtend ? extended : unknown)
    >;
  };

export type ClientExtend = {
  [key: string]: unknown;
};
