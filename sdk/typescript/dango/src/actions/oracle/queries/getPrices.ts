import { queryWasmSmart } from "../../../index.js";
import type { Client } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { Denom } from "../../../types/index.js";
import type { AppConfig } from "../../../types/app.js";
import type { Price } from "../../../types/oracle.js";

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

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  const msg = {
    prices: { startAfter, limit },
  };

  return await queryWasmSmart(client, { contract: addresses.oracle, msg, height });
}
