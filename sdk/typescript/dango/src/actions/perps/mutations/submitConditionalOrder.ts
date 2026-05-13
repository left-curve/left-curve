import { getAppConfig } from "../../../index.js";
import { getAction } from "../../index.js";
import { execute } from "../../app/mutations/execute.js";

import type { Address } from "../../../types/index.js";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type {
  AppConfig,
  Client,
  Signer,
  TriggerDirection,
  TypedDataParameter,
} from "../../../types/index.js";

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

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

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
