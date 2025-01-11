import type { Chain, Client, ClientConfig, RequiredBy, Signer, Transport } from "@left-curve/types";

import { type PublicActions, publicActions } from "../../../core/src/actions/publicActions.js";
import { createBaseClient } from "../../../core/src/clients/baseClient.js";
import { type SignerActions, signerActions } from "../actions/signerActions.js";

export type SignerClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer = Signer,
> = RequiredBy<ClientConfig<transport, chain, signer>, "signer" | "username">;

export type SignerClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer = Signer,
> = Client<
  transport,
  chain,
  signer,
  PublicActions<transport, chain> & SignerActions<transport, chain>
>;

export function createSignerClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
  signer extends Signer = Signer,
>(
  parameters: SignerClientConfig<transport, chain, signer>,
): SignerClient<transport, chain, signer> {
  const { name = "Signer Client", type = "signer" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type,
  });

  return client.extend(publicActions).extend(signerActions);
}
