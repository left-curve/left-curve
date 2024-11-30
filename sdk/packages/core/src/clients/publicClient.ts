import type { Chain, Client, ClientConfig, Transport } from "@left-curve/types";
import type { PublicActions } from "../actions/publicActions.js";

import { publicActions } from "../actions/publicActions.js";
import { createBaseClient } from "./baseClient.js";

export type PublicClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
> = ClientConfig<transport, chain, undefined>;

export type PublicClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
> = Client<transport, chain, undefined, PublicActions<transport, chain>>;

export function createPublicClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
>(parameters: PublicClientConfig<transport, chain>): PublicClient<transport, chain> {
  const { name = "Public Client" } = parameters;
  const client = createBaseClient({
    ...parameters,
    name,
    type: "public",
  });
  return client.extend(publicActions);
}
