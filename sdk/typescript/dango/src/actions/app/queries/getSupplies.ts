import type { Client, Coins, Denom } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

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
export async function getSupplies(
  client: Client,
  parameters: GetSuppliesParameters,
): GetSuppliesReturnType {
  const { limit, startAfter, height = 0 } = parameters || {};
  const query = {
    supplies: { startAfter, limit },
  };

  const res = await queryApp(client, { query, height });

  if ("supplies" in res) return res.supplies;
  throw new Error(`expecting supplies response, got ${JSON.stringify(res)}`);
}
