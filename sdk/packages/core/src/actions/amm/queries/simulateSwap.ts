import type {
  Address,
  AmmQueryMsg,
  Chain,
  Client,
  Coin,
  PoolId,
  Signer,
  SwapOutcome,
  Transport,
} from "@leftcurve/types";
import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

export type SimulateSwapParameters = {
  height?: number;
  route: PoolId[];
  input: Coin;
};

export type SimulateSwapReturnType = Promise<SwapOutcome>;

/**
 * Get the state of a single pool by ID.
 * @param parameters
 * @param parameters.poolId The ID of the pool to query.
 * @param parameters.height The height at which to query the pool's state.
 * @returns The state of the pool
 */
export async function simulateSwap<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateSwapParameters,
): SimulateSwapReturnType {
  const { input, route, height = 0 } = parameters;
  const msg: AmmQueryMsg = { simulate: { input, route: [...new Set(route)] } };

  const ammAddr = await getAppConfig<Address>(client, { key: "amm" });

  return await queryWasmSmart(client, { contract: ammAddr, msg, height });
}
