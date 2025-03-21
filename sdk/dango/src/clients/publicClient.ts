import { createBaseClient } from "@left-curve/sdk";
import { publicActions } from "../actions/index.js";

import type { Transport } from "@left-curve/sdk/types";
import type { DangoClient, PublicClient, PublicClientConfig } from "../types/clients.js";

export function createPublicClient<transport extends Transport>(
  parameters: PublicClientConfig<transport>,
): PublicClient<transport> {
  const { name = "Dango Public Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type: "dango",
  }) as DangoClient<transport>;

  return client.extend(publicActions) as PublicClient<transport>;
}
