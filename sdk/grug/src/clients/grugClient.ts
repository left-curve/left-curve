import type { Chain, Client, ClientConfig, Transport } from "../types/index.js";

import { type GrugActions, grugActions } from "../actions/grugActions.js";
import { createBaseClient } from "./baseClient.js";

export type GrugClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
> = ClientConfig<transport, chain, undefined>;

export type GrugClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
> = Client<transport, chain, undefined, GrugActions<transport, chain>>;

export function createGrugClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
>(parameters: GrugClientConfig<transport, chain>): GrugClient<transport, chain> {
  const { name = "Grug Client" } = parameters;
  const client = createBaseClient({
    ...parameters,
    name,
    type: "grug",
  });
  return client.extend(grugActions);
}
