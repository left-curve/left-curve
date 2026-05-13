import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Coin, DexQueryMsg, SwapRoute } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type SimulateSwapExactAmountOutParameters = {
  output: Coin;
  route: SwapRoute;
  height?: number;
};

export type SimulateSwapExactAmountOutReturnType = Promise<Coin>;

/**
 * Get the exact amount out of a swap.
 * @param parameters
 * @param parameters.output The coin output of the swap.
 * @param parameters.route The route of the swap.
 * @param parameters.height The height at which to query the prices.
 * @returns The prices.
 */
export async function simulateSwapExactAmountOut(
  client: Client,
  parameters: SimulateSwapExactAmountOutParameters,
): SimulateSwapExactAmountOutReturnType {
  const { output, route, height = 0 } = parameters;

  const msg: DexQueryMsg = {
    simulateSwapExactAmountOut: {
      output,
      route,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
