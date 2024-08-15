import type { Account, Chain, Client, ClientConfig, Transport } from "@leftcurve/types";

import { type WalletActions, walletActions } from "./actions/walletAction";
import { createBaseClient } from "./baseClient";

export type WalletClientConfig<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
> = Pick<
  ClientConfig<transport, chain, account>,
  "account" | "batch" | "chain" | "key" | "name" | "transport"
>;

export type WalletClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = undefined,
> = Client<transport, chain, account, WalletActions<transport, chain>>;

export function createWalletClient<
  transport extends Transport,
  chain extends Chain | undefined = undefined,
  account extends Account | undefined = undefined,
>(
  parameters: WalletClientConfig<transport, chain, account>,
): WalletClient<transport, chain, account> {
  const { key = "wallet", name = "Wallet Client" } = parameters;
  const client = createBaseClient({
    ...parameters,
    key,
    name,
    type: "walletClient",
  });
  return client.extend(walletActions) as WalletClient<transport, chain, account>;
}
