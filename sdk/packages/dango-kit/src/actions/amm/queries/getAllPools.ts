import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

import type {
  AmmQueryMsg,
  Chain,
  Client,
  Pool,
  PoolId,
  Signer,
  Transport,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type GetAllPoolsParameters = {
  height?: number;
  startAfter?: PoolId;
  limit?: number;
};

export type GetAllPoolsReturnType = Promise<Record<PoolId, Pool>>;

/**
 * Get the states of all pools.
 * @param parameters
 * @param parameters.startAfter The ID of the pool to start after.
 * @param parameters.limit The maximum number of pools to return.
 * @param parameters.height The height at which to query the pools' states.
 * @returns The states of all pools
 */
export async function getAllPools<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAllPoolsParameters,
): GetAllPoolsReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg: AmmQueryMsg = { pools: { startAfter, limit } };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.amm, msg, height });
}
