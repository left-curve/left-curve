import type { Account, Chain, Client, ClientConfig, Transport } from "@leftcurve/types";
import type { PublicActions } from "./actions/publicActions";

import { publicActions } from "./actions/publicActions";
import { createBaseClient } from "./baseClient";

export type PublicClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
> = Pick<ClientConfig<transport, chain, account>, "batch" | "chain" | "key" | "name" | "transport">;

export type PublicClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
> = Client<transport, chain, account, PublicActions<transport, chain>>;

export function createPublicClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
  account extends Account | undefined = undefined,
>(
  parameters: PublicClientConfig<transport, chain, account>,
): PublicClient<transport, chain, account> {
  const { key = "public", name = "Public Client" } = parameters;
  const client = createBaseClient({
    ...parameters,
    key,
    name,
    type: "publicClient",
  });
  return client.extend(publicActions) as PublicClient<transport, chain, account>;
}
