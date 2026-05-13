import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Denom, Price } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type GetPricesParameters = {
  startAfter?: Denom;
  limit?: number;
  height?: number;
};

export type GetPricesReturnType = Promise<Record<Denom, Price>>;

/**
 * Get the votes for a proposal.
 * @param parameters
 * @param parameters.startAfter The denom to start after.
 * @param parameters.limit The maximum number of prices to return.
 * @param parameters.height The height at which to query the prices.
 * @returns The prices.
 */
export async function getPrices(
  client: Client,
  parameters: GetPricesParameters = {},
): GetPricesReturnType {
  const { startAfter, limit, height = 0 } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg = {
    prices: { startAfter, limit },
  };

  return await queryWasmSmart(client, { contract: addresses.oracle, msg, height });
}
