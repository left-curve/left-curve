import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type {
  AppConfig,
  ChildOrder,
  DangoClient,
  PerpsOrderKind,
  Signer,
  TypedDataParameter,
} from "../../../types/index.js";

export type SubmitPerpsOrderParameters = {
  sender: Address;
  pairId: string;
  size: string;
  kind: PerpsOrderKind;
  reduceOnly: boolean;
  tp?: ChildOrder;
  sl?: ChildOrder;
};

export type SubmitPerpsOrderReturnType = SignAndBroadcastTxReturnType;

export async function submitPerpsOrder<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SubmitPerpsOrderParameters,
): SubmitPerpsOrderReturnType {
  const { sender, pairId, size, kind, reduceOnly, tp, sl } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

  const buildChildOrder = (child: ChildOrder) => ({
    triggerPrice: child.triggerPrice,
    maxSlippage: child.maxSlippage,
    ...(child.size ? { size: child.size } : {}),
  });

  // Strip a `null` / `undefined` `clientOrderId` from the limit body so
  // the JSON message and the EIP-712 typed-data structure agree on
  // whether the field is present. The typed-data builder below uses the
  // same `!= null` check.
  const limitHasClientOrderId = "limit" in kind && kind.limit.clientOrderId != null;
  const normalizedKind: PerpsOrderKind =
    "limit" in kind
      ? {
          limit: {
            limitPrice: kind.limit.limitPrice,
            timeInForce: kind.limit.timeInForce,
            ...(limitHasClientOrderId ? { clientOrderId: kind.limit.clientOrderId as string } : {}),
          },
        }
      : kind;

  const msg = {
    trade: {
      submitOrder: {
        pairId,
        size,
        kind: normalizedKind,
        reduceOnly,
        ...(tp ? { tp: buildChildOrder(tp) } : {}),
        ...(sl ? { sl: buildChildOrder(sl) } : {}),
      },
    },
  };

  const kindTypedData =
    "market" in kind
      ? {
          kind: [{ name: "market", type: "Market" }],
          Market: [{ name: "max_slippage", type: "string" }],
        }
      : {
          kind: [{ name: "limit", type: "Limit" }],
          Limit: [
            { name: "limit_price", type: "string" },
            { name: "time_in_force", type: "string" },
            ...(limitHasClientOrderId ? [{ name: "client_order_id", type: "string" }] : []),
          ],
        };

  const childOrderTypeFor = (child: ChildOrder) => [
    { name: "trigger_price", type: "string" },
    { name: "max_slippage", type: "string" },
    ...(child.size ? [{ name: "size", type: "string" }] : []),
  ];

  const submitOrderFields = [
    { name: "pair_id", type: "string" },
    { name: "size", type: "string" },
    { name: "kind", type: "Kind" },
    { name: "reduce_only", type: "bool" },
    ...(tp ? [{ name: "tp", type: "ChildOrderTp" }] : []),
    ...(sl ? [{ name: "sl", type: "ChildOrderSl" }] : []),
  ];

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "submit_order", type: "SubmitOrder" }],
      SubmitOrder: submitOrderFields,
      Kind: kindTypedData.kind,
      ...(kindTypedData.Market ? { Market: kindTypedData.Market } : {}),
      ...(kindTypedData.Limit ? { Limit: kindTypedData.Limit } : {}),
      ...(tp ? { ChildOrderTp: childOrderTypeFor(tp) } : {}),
      ...(sl ? { ChildOrderSl: childOrderTypeFor(sl) } : {}),
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
