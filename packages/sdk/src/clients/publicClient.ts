import type { Chain, Client, ClientConfig, Transport } from "@leftcurve/types";
import type { PublicActions } from "../actions/publicActions";

import { publicActions } from "../actions/publicActions";
import { createBaseClient } from "./baseClient";

export type PublicClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
> = ClientConfig<transport, chain, undefined>;

export type PublicClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
> = Omit<
  Client<transport, chain, undefined, PublicActions<transport, chain>>,
  | "batch"
  | "uid"
  | "extend"
  | "transport"
  | "type"
  | "name"
  | "key"
  | "chain"
  | "signer"
  | "broadcast"
  | "query"
>;

export function createPublicClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
>(parameters: PublicClientConfig<transport, chain>): PublicClient<transport, chain> {
  const { key = "public", name = "Public Client" } = parameters;
  const client = createBaseClient({
    ...parameters,
    key,
    name,
    type: "publicClient",
  });
  return client.extend(publicActions);
}
