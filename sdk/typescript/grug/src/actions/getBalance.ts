import type { Address, Chain, Client, Denom, Signer, Transport } from "../types/index.js";
import { getAction } from "./getAction.js";
import { queryApp } from "./queryApp.js";

export type GetBalanceParameters = {
  address: Address;
  denom: Denom;
  height?: number;
};

export type GetBalanceReturnType = Promise<number>;

/**
 * Get the balance of an account.
 * @param parameters
 * @param parameters.address The address to get the balance of.
 * @param parameters.denom The denomination of the token.
 * @param parameters.height The height at which to query the balance.
 * @returns The balance of the account.
 */
export async function getBalance<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetBalanceParameters,
): GetBalanceReturnType {
  const { address, denom, height = 0 } = parameters;
  const query = {
    balance: { address, denom },
  };
  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if (!("balance" in res)) {
    throw new Error(`expecting balance response, got ${JSON.stringify(res)}`);
  }

  return Number.parseInt(res.balance.amount);
}
