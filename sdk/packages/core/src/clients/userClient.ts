import type { Chain, Client, ClientConfig, RequiredBy, Signer, Transport } from "@leftcurve/types";

import { type PublicActions, publicActions } from "../actions/publicActions.js";
import { type UserActions, userActions } from "../actions/userActions.js";
import { createBaseClient } from "./baseClient.js";

export type UserClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer = Signer,
> = RequiredBy<ClientConfig<transport, chain, signer>, "signer" | "username">;

export type UserClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer = Signer,
> = Client<
  transport,
  chain,
  signer,
  PublicActions<transport, chain> & UserActions<transport, chain>
>;

export function createUserClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
  signer extends Signer = Signer,
>(parameters: UserClientConfig<transport, chain, signer>): UserClient<transport, chain, signer> {
  const { name = "Wallet Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    name,
    type: "user",
  });

  return client.extend(publicActions).extend(userActions);
}
