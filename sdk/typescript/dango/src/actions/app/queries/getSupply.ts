import type { Client, Coin, Denom } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type GetSupplyParameters = {
  denom: Denom;
};

export type GetSupplyReturnType = Promise<Coin>;

/**
 * Get the supply of a token.
 * @param parameters
 * @param parameters.denom The denomination of the token.
 * @returns The supply of the token.
 */
export async function getSupply(
  client: Client,
  parameters: GetSupplyParameters,
): GetSupplyReturnType {
  const { denom } = parameters;
  const query = {
    supply: { denom },
  };

  const res = await queryApp(client, { query });

  if ("supply" in res) return res.supply;
  throw new Error(`expecting supply response, got ${JSON.stringify(res)}`);
}
