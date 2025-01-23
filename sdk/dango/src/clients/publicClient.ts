import { createBaseClient } from "@left-curve/sdk";
import { publicActions } from "../actions/index.js";

import type { Transport } from "@left-curve/sdk/types";
import type { Chain } from "../types/chain.js";
import type { PublicClient, PublicClientConfig } from "../types/clients.js";
import type { Signer } from "../types/signer.js";

export function createPublicClient<transport extends Transport>(
  parameters: PublicClientConfig<transport>,
): PublicClient<transport> {
  const { name = "Dango Public Client" } = parameters;

  const client = createBaseClient<transport, Chain, Signer>({
    ...parameters,
    name,
    type: "dango",
  });

  return client.extend(publicActions) as unknown as PublicClient<transport>;
}
