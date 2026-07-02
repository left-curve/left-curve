import type { Address, Client, Denom } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type GetBalanceParameters = {
  address: Address;
  denom: Denom;
};

export type GetBalanceReturnType = Promise<string>;

/**
 * Get the balance of an account.
 * @param parameters
 * @param parameters.address The address to get the balance of.
 * @param parameters.denom The denomination of the token.
 * @returns The balance of the account as a base-unit string.
 */
export async function getBalance(
  client: Client,
  parameters: GetBalanceParameters,
): GetBalanceReturnType {
  const { address, denom } = parameters;
  const query = {
    balance: { address, denom },
  };
  const res = await queryApp(client, { query });

  if (!("balance" in res)) {
    throw new Error(`expecting balance response, got ${JSON.stringify(res)}`);
  }

  return res.balance.amount;
}
