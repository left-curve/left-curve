import type { Address, Chain, Client, Coins, Denom, Signer, Transport } from "../types/index.js";
import { getAction } from "./getAction.js";
import { queryApp } from "./queryApp.js";

export type GetBalancesParameters = {
  address: Address;
  startAfter?: Denom;
  limit?: number;
  height?: number;
};

export type GetBalancesReturnType = Promise<Coins>;

/**
 * Get the balances.
 * @param parameters
 * @param parameters.address The address to get the balances of.
 * @param parameters.startAfter The token to start after.
 * @param parameters.limit The number of tokens to return.
 * @param parameters.height The height at which to query the balances.
 * @returns The balances.
 */
export async function getBalances<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetBalancesParameters,
): GetBalancesReturnType {
  const { address, startAfter, limit, height = 0 } = parameters;
  const query = {
    balances: { address, startAfter, limit },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if (!("balances" in res)) {
    throw new Error(`expecting balances response, got ${JSON.stringify(res)}`);
  }

  return res.balances;
}
