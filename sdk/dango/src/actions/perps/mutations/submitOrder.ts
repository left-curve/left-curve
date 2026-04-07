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

  const msg = {
    trade: {
      submitOrder: {
        pairId,
        size,
        kind,
        reduceOnly,
        ...(tp ? { tp: buildChildOrder(tp) } : {}),
        ...(sl ? { sl: buildChildOrder(sl) } : {}),
      },
    },
  };

  const kindTypedData = "market" in kind
    ? {
        kind: [{ name: "market", type: "Market" }],
        Market: [{ name: "maxSlippage", type: "string" }],
      }
    : {
        kind: [{ name: "limit", type: "Limit" }],
        Limit: [
          { name: "limitPrice", type: "string" },
          { name: "postOnly", type: "bool" },
        ],
      };

  const childOrderTypeFor = (child: ChildOrder) => [
    { name: "triggerPrice", type: "string" },
    { name: "maxSlippage", type: "string" },
    ...(child.size ? [{ name: "size", type: "string" }] : []),
  ];

  const submitOrderFields = [
    { name: "pairId", type: "string" },
    { name: "size", type: "string" },
    { name: "kind", type: "Kind" },
    { name: "reduceOnly", type: "bool" },
    ...(tp ? [{ name: "tp", type: "ChildOrderTp" }] : []),
    ...(sl ? [{ name: "sl", type: "ChildOrderSl" }] : []),
  ];

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "submitOrder", type: "SubmitOrder" }],
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
