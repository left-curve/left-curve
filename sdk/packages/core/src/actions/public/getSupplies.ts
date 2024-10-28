import type { Chain, Client, Coins, Denom, Signer, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetSuppliesParameters =
  | {
      startAfter?: Denom;
      limit?: number;
      height?: number;
    }
  | undefined;

export type GetSuppliesReturnType = Promise<Coins>;

/**
 * Get the supplies of all tokens.
 * @param parameters
 * @param parameters.startAfter The token to start after.
 * @param parameters.limit The number of tokens to return.
 * @param parameters.height The height at which to query the supplies.
 * @returns The supplies of all tokens.
 */
export async function getSupplies<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetSuppliesParameters,
): GetSuppliesReturnType {
  const { limit, startAfter, height = 0 } = parameters || {};
  const query = {
    supplies: { startAfter, limit },
  };
  const res = await queryApp<chain, signer>(client, { query, height });

  if ("supplies" in res) return res.supplies;
  throw new Error(`expecting supplies response, got ${JSON.stringify(res)}`);
}
