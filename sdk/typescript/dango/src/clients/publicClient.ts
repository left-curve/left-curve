import { createBaseClient } from "./baseClient.js";
import { publicActions } from "#actions/publicActions.js";

import type { Client, PublicClientConfig } from "@left-curve/types";
import type { PublicActions } from "#actions/publicActions.js";

export type PublicClient = Client<undefined, PublicActions>;

export function createPublicClient(parameters: PublicClientConfig): PublicClient {
  const { name = "Public Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type: "dango",
  }) as Client;

  return client.extend(publicActions) as PublicClient;
}
