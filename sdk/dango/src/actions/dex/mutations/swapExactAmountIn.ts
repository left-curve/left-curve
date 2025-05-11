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

export type SwapExactAmountInParameters = {
  sender: Address;
  route: SwapRoute;
  minimumOutput?: string;
  input: Coin;
};

export type SwapExactAmountInReturnType = BroadcastTxSyncReturnType;

export async function swapExactAmountIn<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SwapExactAmountInParameters,
): SwapExactAmountInReturnType {
  const { route, minimumOutput, sender, input } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg: DexExecuteMsg = {
    swapExactAmountIn: {
      route,
      minimumOutput,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "swap_exact_amount_in", type: "SwapExactAmountIn" }],
    extraTypes: {
      SwapExactAmountIn: [
        { name: "route", type: "SwapRoute[]" },
        ...(minimumOutput ? [{ name: "minimum_output", type: "string" }] : []),
      ],
      SwapRoute: [
        { name: "base_denom", type: "string" },
        { name: "quote_denom", type: "string" },
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
