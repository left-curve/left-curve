import type {
  Account,
  Chain,
  Client,
  ClientConfig,
  RequiredBy,
  Signer,
  Transport,
} from "@leftcurve/types";

import { type PublicActions, publicActions } from "../actions/publicActions";
import { type UserActions, userActions } from "../actions/userActions";
import { createBaseClient } from "./baseClient";

export type UserClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer = Signer,
> = RequiredBy<ClientConfig<transport, chain, signer>, "signer">;

export type UserClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer = Signer,
> = Omit<
  Client<transport, chain, signer, PublicActions<transport, chain> & UserActions<transport, chain>>,
  | "batch"
  | "uid"
  | "extend"
  | "transport"
  | "type"
  | "name"
  | "key"
  | "chain"
  | "signer"
  | "broadcast"
  | "query"
>;

export function createUserClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
  signer extends Signer = Signer,
>(parameters: UserClientConfig<transport, chain, signer>): UserClient<transport, chain, signer> {
  const { key = "wallet", name = "Wallet Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    key,
    name,
    type: "walletClient",
  });

  return client.extend(publicActions).extend(userActions);
}
