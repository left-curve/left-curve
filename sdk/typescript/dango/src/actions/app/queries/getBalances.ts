import type { Address, Client, Coins, Denom } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type GetBalancesParameters = {
  address: Address;
  startAfter?: Denom;
  limit?: number;
};

export type GetBalancesReturnType = Promise<Coins>;

/**
 * Get the balances.
 * @param parameters
 * @param parameters.address The address to get the balances of.
 * @param parameters.startAfter The token to start after.
 * @param parameters.limit The number of tokens to return.
 * @returns The balances.
 */
export async function getBalances(
  client: Client,
  parameters: GetBalancesParameters,
): GetBalancesReturnType {
  const { address, startAfter, limit } = parameters;
  const query = {
    balances: { address, startAfter, limit },
  };

  const res = await queryApp(client, { query });

  if (!("balances" in res)) {
    throw new Error(`expecting balances response, got ${JSON.stringify(res)}`);
  }

  return res.balances;
}
