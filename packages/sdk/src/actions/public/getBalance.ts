import type { Account, Chain, Client, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetBalanceParameters = {
  address: string;
  denom: string;
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
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetBalanceParameters,
): GetBalanceReturnType {
  const { address, denom, height = 0 } = parameters;
  const query = {
    balance: { address, denom },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if (!("balance" in res)) {
    throw new Error(`expecting balance response, got ${JSON.stringify(res)}`);
  }

  return Number.parseInt(res.balance.amount);
}
