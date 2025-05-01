import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Coin, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "#types/app.js";
import type { DexQueryMsg, SwapRoute } from "#types/dex.js";

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
export async function simulateSwapExactAmountIn<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateSwapExactAmountInParameters,
): SimulateSwapExactAmountInReturnType {
  const { input, route, height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: DexQueryMsg = {
    simulateSwapExactAmountIn: {
      input,
      route,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
