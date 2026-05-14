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

  const { addresses } = await getAppConfig(client);

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
