import type { Account, Chain, Client, ClientConfig, Transport } from "@leftcurve/types";

import { type PublicActions, publicActions } from "../actions/publicActions";
import { type UserActions, userActions } from "../actions/userActions";
import { createBaseClient } from "./baseClient";

export type UserClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account = Account,
> = ClientConfig<transport, chain, account>;

export type UserClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account = Account,
> = Omit<
  Client<
    transport,
    chain,
    account,
    PublicActions<transport, chain> & UserActions<transport, chain>
  >,
  | "batch"
  | "uid"
  | "extend"
  | "transport"
  | "type"
  | "name"
  | "key"
  | "chain"
  | "account"
  | "broadcast"
  | "query"
>;

export function createUserClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
  account extends Account = Account,
>(parameters: UserClientConfig<transport, chain, account>): UserClient<transport, chain, account> {
  const { key = "wallet", name = "Wallet Client" } = parameters;

  const client = createBaseClient({
    ...parameters,
    key,
    name,
    type: "walletClient",
  });

  return client.extend(publicActions).extend(userActions);
}
