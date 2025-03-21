import { createBaseClient } from "@left-curve/sdk";
import { publicActions, signerActions } from "../actions/index.js";

import type { Transport } from "@left-curve/sdk/types";

import type { SignerClient, SignerClientConfig } from "../types/clients.js";

export function createSignerClient<transport extends Transport = Transport>(
  parameters: SignerClientConfig<transport>,
): SignerClient<transport> {
  const { name = "Dango Signer Client", type = "dango", username } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type,
    username,
  }) as SignerClient<transport>;

  return client.extend(publicActions).extend(signerActions);
}
