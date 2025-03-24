import { createBaseClient } from "@left-curve/sdk";
import { publicActions, signerActions } from "../actions/index.js";

import type { Client, Transport } from "@left-curve/sdk/types";

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
  }) as unknown as Client<transport>;

  const publicClient = client.extend(publicActions) as unknown as SignerClient<transport>;
  return publicClient.extend(signerActions) as SignerClient<transport>;
}
