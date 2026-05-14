import type { Chain } from "./chain.js";
import type { Signer } from "./signer.js";
import type { RequestFn, SubscribeFn, Transport } from "./transports.js";

export type ClientConfig<
  signer extends Signer | undefined = undefined,
  custom extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
> = {
  signer?: signer;
  chain: Chain;
  name?: string;
  type?: string | undefined;
  transport: Transport;
} & custom;

export type Client<
  signer extends Signer | undefined = Signer | undefined,
  extended extends ClientExtend | undefined = ClientExtend | undefined,
> = {
  signer: signer;
  chain: Chain;
  name: string;
  request: RequestFn;
  subscribe: SubscribeFn;
  type: string;
  uid: string;
} & (extended extends ClientExtend ? extended : unknown) & {
    extend: <const client extends ClientExtend = ClientExtend>(
      fn: (client: Client<signer, extended>) => client,
    ) => Client<signer, client & (extended extends ClientExtend ? extended : unknown)>;
  };

export type ClientExtend = {
  [key: string]: unknown;
};
