import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address, Coin } from "@left-curve/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";
import type {
  Client,
  DexExecuteMsg,
  Signer,
  SwapRoute,
  TypedDataParameter,
} from "@left-curve/types";

export type SwapExactAmountOutParameters = {
  sender: Address;
  route: SwapRoute;
  output: Coin;
  input: Coin;
};

export type SwapExactAmountOutReturnType = BroadcastTxSyncReturnType;

export async function swapExactAmountOut(
  client: Client<Signer>,
  parameters: SwapExactAmountOutParameters,
): SwapExactAmountOutReturnType {
  const { route, output, sender, input } = parameters;

  const { addresses } = await getAppConfig(client);

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
