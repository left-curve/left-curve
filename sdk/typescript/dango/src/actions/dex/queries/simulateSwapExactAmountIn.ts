import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Coin, DexQueryMsg, SwapRoute } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type SimulateSwapExactAmountInParameters = {
  input: Coin;
  route: SwapRoute;
  height?: number;
};

export type SimulateSwapExactAmountInReturnType = Promise<Coin>;

/**
 * Get the exact amount in of a swap.
 * @param parameters
 * @param parameters.input The coin input of the swap.
 * @param parameters.route The route of the swap.
 * @param parameters.height The height at which to query the prices.
 * @returns The prices.
 */
export async function simulateSwapExactAmountIn(
  client: Client,
  parameters: SimulateSwapExactAmountInParameters,
): SimulateSwapExactAmountInReturnType {
  const { input, route, height = 0 } = parameters;

  const msg: DexQueryMsg = {
    simulateSwapExactAmountIn: {
      input,
      route,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
