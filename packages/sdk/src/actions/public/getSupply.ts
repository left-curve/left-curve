import type { Chain, Client, Signer, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetSupplyParameters = {
  denom: string;
  height?: number;
};

export type GetSupplyReturnType = Promise<number>;

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
  const res = await queryApp<chain, signer>(client, { query, height });

  if ("supply" in res) return Number.parseInt(res.supply.amount);
  throw new Error(`expecting supply response, got ${JSON.stringify(res)}`);
}
