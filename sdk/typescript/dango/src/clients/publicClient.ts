import { createBaseClient } from "./baseClient.js";
import { publicActions } from "../actions/publicActions.js";

import type { Client } from "../types/client.js";
import type { PublicClient, PublicClientConfig } from "../types/clients.js";

export function createPublicClient(parameters: PublicClientConfig): PublicClient {
  const { name = "Public Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type: "dango",
  }) as Client;

  return client.extend(publicActions) as PublicClient;
}
