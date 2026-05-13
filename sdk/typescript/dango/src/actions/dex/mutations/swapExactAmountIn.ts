import { getAppConfig } from "../../../index.js";
import { getAction } from "../../index.js";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Coin } from "../../../types/index.js";
import type { BroadcastTxSyncReturnType } from "../../app/mutations/broadcastTxSync.js";
import type {
  AppConfig,
  Client,
  DexExecuteMsg,
  Signer,
  SwapRoute,
  TypedDataParameter,
} from "../../../types/index.js";

export type SwapExactAmountInParameters = {
  sender: Address;
  route: SwapRoute;
  minimumOutput?: string;
  input: Coin;
};

export type SwapExactAmountInReturnType = BroadcastTxSyncReturnType;

export async function swapExactAmountIn(
  client: Client<Signer>,
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
