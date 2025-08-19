import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Coin, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { DexQueryMsg, SwapRoute } from "../../../types/dex.js";

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
export async function simulateSwapExactAmountOut<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateSwapExactAmountOutParameters,
): SimulateSwapExactAmountOutReturnType {
  const { output, route, height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: DexQueryMsg = {
    simulateSwapExactAmountOut: {
      output,
      route,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
