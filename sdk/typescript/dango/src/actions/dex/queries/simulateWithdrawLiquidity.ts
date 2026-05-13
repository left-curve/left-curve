import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, CoinPair, GetDexQueryMsg, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetDexQueryMsg<"simulateWithdrawLiquidity">;

export type SimulateWithdrawLiquidityParameters = Prettify<
  {
    height?: number;
  } & ActionMsg["simulateWithdrawLiquidity"]
>;

export type SimulateWithdrawLiquidityReturnType = Promise<CoinPair>;

/**
 * Simulates withdrawing liquidity from a trading pair.
 * @param parameters
 * @param parameters.baseDenom - The base denomination of the trading pair.
 * @param parameters.quoteDenom - The quote denomination of the trading pair.
 * @param parameters.lpBurnAmount - The amount of LP tokens to burn.
 * @param parameters.height - The block height to query at (default is the latest block).
 * @returns The amount of base and quote tokens that would be received from the withdrawal.
 */
export async function simulateWithdrawLiquidity(
  client: Client,
  parameters: SimulateWithdrawLiquidityParameters,
): SimulateWithdrawLiquidityReturnType {
  const { baseDenom, quoteDenom, lpBurnAmount, height = 0 } = parameters;

  const msg: ActionMsg = {
    simulateWithdrawLiquidity: {
      baseDenom,
      quoteDenom,
      lpBurnAmount,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
