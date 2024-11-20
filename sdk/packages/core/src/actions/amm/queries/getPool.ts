import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

import type { AmmQueryMsg, Chain, Client, Pool, PoolId, Signer, Transport } from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

export type GetPoolParameters = {
  height?: number;
  poolId: PoolId;
};

export type GetPoolReturnType = Promise<Pool>;

/**
 * Get the state of a single pool by ID.
 * @param parameters
 * @param parameters.poolId The ID of the pool to query.
 * @param parameters.height The height at which to query the pool's state.
 * @returns The state of the pool
 */
export async function getPool<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetPoolParameters,
): GetPoolReturnType {
  const { poolId, height = 0 } = parameters;
  const msg: AmmQueryMsg = { pool: { poolId } };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.amm, msg, height });
}
