import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, DexQueryMsg, PairId, PairUpdate } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type GetPairsParameters = {
  limit?: number;
  startAfter?: PairId;
  height?: number;
};

export type GetPairsReturnType = Promise<PairUpdate[]>;

/**
 * Enumerate all trading pairs and their parameters.
 * @param parameters
 * @param parameters.limit The maximum number of pairs to return.
 * @param parameters.startAfter The ID of the pair to start after.
 * @param parameters.height The height at which to query the pairs
 * @returns The pairs and their parameters.
 */
export async function getPairs(
  client: Client,
  parameters: GetPairsParameters = {},
): GetPairsReturnType {
  const { limit, startAfter, height = 0 } = parameters;

  const msg: DexQueryMsg = {
    pairs: {
      limit,
      startAfter,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
