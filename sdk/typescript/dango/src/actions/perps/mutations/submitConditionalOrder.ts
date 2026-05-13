import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address } from "@left-curve/types";
import type { SignAndBroadcastTxReturnType } from "#actions/app/mutations/signAndBroadcastTx.js";
import type { Client, Signer, TriggerDirection, TypedDataParameter } from "@left-curve/types";

export type SubmitConditionalOrderParameters = {
  sender: Address;
  pairId: string;
  size?: string;
  triggerPrice: string;
  triggerDirection: TriggerDirection;
  maxSlippage: string;
};

export type SubmitConditionalOrderReturnType = SignAndBroadcastTxReturnType;

export async function submitConditionalOrder(
  client: Client<Signer>,
  parameters: SubmitConditionalOrderParameters,
): SubmitConditionalOrderReturnType {
  const { sender, pairId, size, triggerPrice, triggerDirection, maxSlippage } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg = {
    trade: {
      submitConditionalOrder: {
        pairId,
        ...(size !== undefined ? { size } : {}),
        triggerPrice,
        triggerDirection,
        maxSlippage,
      },
    },
  };

  const submitConditionalOrderFields = [
    { name: "pair_id", type: "string" },
    ...(size !== undefined ? [{ name: "size", type: "string" }] : []),
    { name: "trigger_price", type: "string" },
    { name: "trigger_direction", type: "string" },
    { name: "max_slippage", type: "string" },
  ];

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "submit_conditional_order", type: "SubmitConditionalOrder" }],
      SubmitConditionalOrder: submitConditionalOrderFields,
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg,
      typedData,
      contract: addresses.perps,
    },
  });
}
