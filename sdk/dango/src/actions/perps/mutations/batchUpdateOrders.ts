import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type {
  AppConfig,
  DangoClient,
  PerpsSubmitOrCancelOrderRequest,
  Signer,
  TypedDataParameter,
  TypedDataProperty,
} from "../../../types/index.js";

export type BatchUpdatePerpsOrdersParameters = {
  sender: Address;
  /**
   * Ordered list of submit / cancel actions applied atomically. Must be
   * non-empty and no longer than `Param.maxActionBatchSize`; the chain
   * rejects oversize batches before any action runs.
   *
   * Conditional (TP/SL) orders are not supported in batches.
   */
  actions: PerpsSubmitOrCancelOrderRequest[];
};

export type BatchUpdatePerpsOrdersReturnType = SignAndBroadcastTxReturnType;

/**
 * Send a `TraderMsg::BatchUpdateOrders` execute message.
 *
 * Actions execute sequentially — later actions observe the state
 * written by earlier ones — and atomically: if any action fails, the
 * message reverts and no partial state is persisted.
 */
export async function batchUpdatePerpsOrders<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: BatchUpdatePerpsOrdersParameters,
): BatchUpdatePerpsOrdersReturnType {
  const { sender, actions } = parameters;

  if (actions.length === 0) {
    throw new Error("batchUpdatePerpsOrders requires at least one action");
  }

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    trade: {
      batchUpdateOrders: actions,
    },
  };

  // EIP-712 types for each variant used in the batch.
  const extraTypes: Record<string, TypedDataProperty[]> = {
    Trade: [{ name: "batch_update_orders", type: "BatchAction[]" }],
    BatchAction: [
      { name: "submit", type: "SubmitAction" },
      { name: "cancel", type: "CancelAction" },
    ],
    SubmitAction: [
      { name: "pair_id", type: "string" },
      { name: "size", type: "string" },
      { name: "kind", type: "string" },
      { name: "reduce_only", type: "bool" },
    ],
    CancelAction: [{ name: "value", type: "string" }],
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes,
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
