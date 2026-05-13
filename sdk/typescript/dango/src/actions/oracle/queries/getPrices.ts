import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Denom, Signer } from "@left-curve/sdk/types";
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
export async function getPrices<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
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
