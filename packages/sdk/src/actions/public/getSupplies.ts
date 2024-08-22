import type { Account, Chain, Client, Coin, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetSuppliesParameters =
  | {
      startAfter?: string;
      limit?: number;
      height?: number;
    }
  | undefined;

export type GetSuppliesReturnType = Promise<Coin>;

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
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetSuppliesParameters,
): GetSuppliesReturnType {
  const { limit, startAfter, height = 0 } = parameters || {};
  const query = {
    supplies: { startAfter, limit },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if ("supplies" in res) return res.supplies;
  throw new Error(`expecting supplies response, got ${JSON.stringify(res)}`);
}
