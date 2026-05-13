import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type {
  AppConfig,
  DangoClient,
  Signer,
  TriggerDirection,
  TypedDataParameter,
} from "../../../types/index.js";

export type SubmitConditionalOrderInput = {
  pairId: string;
  size?: string;
  triggerPrice: string;
  triggerDirection: TriggerDirection;
  maxSlippage: string;
};

export type SubmitConditionalOrdersParameters = {
  sender: Address;
  orders: SubmitConditionalOrderInput[];
};

export type SubmitConditionalOrdersReturnType = SignAndBroadcastTxReturnType;

export async function submitConditionalOrders<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SubmitConditionalOrdersParameters,
): SubmitConditionalOrdersReturnType {
  const { sender, orders } = parameters;

  if (orders.length === 0) {
    throw new Error("submitConditionalOrders requires at least one order");
  }

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

  const executeMsgs = orders.map((order) => {
    const { pairId, size, triggerPrice, triggerDirection, maxSlippage } = order;

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

    return {
      msg,
      typedData,
      contract: addresses.perps,
    };
  });

  return await execute(client, {
    sender,
    execute: executeMsgs,
  });
}
