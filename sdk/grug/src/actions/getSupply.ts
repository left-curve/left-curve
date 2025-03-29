import type { Chain, Client, Coin, Denom, Signer, Transport } from "../types/index.js";
import { getAction } from "./getAction.js";
import { queryApp } from "./queryApp.js";

export type GetSupplyParameters = {
  denom: Denom;
  height?: number;
};

export type GetSupplyReturnType = Promise<Coin>;

/**
 * Get the supply of a token.
 * @param parameters
 * @param parameters.denom The denomination of the token.
 * @param parameters.height The height at which to query the supply.
 * @returns The supply of the token.
 */
export async function getSupply<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetSupplyParameters,
): GetSupplyReturnType {
  const { denom, height = 0 } = parameters;
  const query = {
    supply: { denom },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if ("supply" in res) return res.supply;
  throw new Error(`expecting supply response, got ${JSON.stringify(res)}`);
}
