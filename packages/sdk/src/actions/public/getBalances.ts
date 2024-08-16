import type { Account, Chain, Client, Coin, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetBalancesParameters = {
  address: string;
  startAfter?: string;
  limit?: number;
  height?: number;
};

export type GetBalancesReturnType = Promise<Coin>;

export async function getBalances<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetBalancesParameters,
): GetBalancesReturnType {
  const { address, startAfter, limit, height = 0 } = parameters;
  const query = {
    balances: { address, startAfter, limit },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if (!("balances" in res)) {
    throw new Error(`expecting balances response, got ${JSON.stringify(res)}`);
  }

  return res.balances;
}
