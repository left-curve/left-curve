import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Denom, DexQueryMsg, PairParams } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type GetPairParameters = {
  quoteDenom: Denom;
  baseDenom: Denom;
  height?: number;
};

export type GetPairReturnType = Promise<PairParams>;

/**
 * Query the parameters of a single trading pair.
 * @param parameters
 * @param parameters.quoteDenom The quote denomination of the pair.
 * @param parameters.baseDenom The base denomination of the pair.
 * @param parameters.height The height at which to query the pairs
 * @returns The prices.
 */
export async function getPair(client: Client, parameters: GetPairParameters): GetPairReturnType {
  const { quoteDenom, baseDenom, height = 0 } = parameters;

  const msg: DexQueryMsg = {
    pair: {
      quoteDenom,
      baseDenom,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
