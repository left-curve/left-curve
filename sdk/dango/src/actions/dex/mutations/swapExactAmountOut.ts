import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "#actions/app/index.js";

import type { Address, Coin, Transport } from "@left-curve/sdk/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";
import type {
  AppConfig,
  DangoClient,
  DexExecuteMsg,
  Signer,
  SwapRoute,
  TypedDataParameter,
} from "#types/index.js";

export type SwapExactAmountOutParameters = {
  sender: Address;
  route: SwapRoute;
  output: Coin;
  input: Coin;
};

export type SwapExactAmountOutReturnType = BroadcastTxSyncReturnType;

export async function swapExactAmountOut<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SwapExactAmountOutParameters,
): SwapExactAmountOutReturnType {
  const { route, output, sender, input } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg: DexExecuteMsg = {
    swapExactAmountOut: {
      route,
      output,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "swap_exact_amount_out", type: "SwapExactAmountOut" }],
    extraTypes: {
      SwapExactAmountOut: [
        { name: "route", type: "SwapRoute[]" },
        { name: "output", type: "OutputCoin" },
      ],
      SwapRoute: [
        { name: "base_denom", type: "string" },
        { name: "quote_denom", type: "string" },
      ],
      OutputCoin: [
        { name: "denom", type: "string" },
        { name: "amount", type: "string" },
      ],
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg,
      typedData,
      contract: addresses.dex,
      funds: {
        [input.denom]: input.amount,
      },
    },
  });
}
