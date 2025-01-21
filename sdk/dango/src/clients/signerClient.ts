import { createBaseClient } from "@left-curve/sdk";
import { publicActions, signerActions } from "../actions/index.js";

import type { Transport } from "@left-curve/types";

import type { SignerClient, SignerClientConfig } from "../types/index.js";

export function createSignerClient<transport extends Transport = Transport>(
  parameters: SignerClientConfig<transport>,
): SignerClient<transport> {
  const { name = "Dango Signer Client", type = "dango", username } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type,
  });

  return client
    .extend(publicActions)
    .extend(signerActions)
    .extend((_) => ({ username }));
}
