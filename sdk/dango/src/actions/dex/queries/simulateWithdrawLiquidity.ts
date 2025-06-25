import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "#types/app.js";
import type { CoinPair, GetDexQueryMsg } from "#types/dex.js";

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
export async function simulateWithdrawLiquidity<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateWithdrawLiquidityParameters,
): SimulateWithdrawLiquidityReturnType {
  const { baseDenom, quoteDenom, lpBurnAmount, height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    simulateWithdrawLiquidity: {
      baseDenom,
      quoteDenom,
      lpBurnAmount,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
