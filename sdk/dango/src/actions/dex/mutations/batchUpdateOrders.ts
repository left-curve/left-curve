import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Coins, Json, Prettify, Transport } from "@left-curve/sdk/types";
import type { BroadcastTxSyncReturnType } from "../../app/mutations/broadcastTxSync.js";

import type { AppConfig } from "../../../types/app.js";
import type { DangoClient } from "../../../types/clients.js";
import type { GetDexExecuteMsg } from "../../../types/dex.js";
import type { Signer } from "../../../types/signer.js";
import type { TypedDataParameter } from "../../../types/typedData.js";

type ActionMsg = GetDexExecuteMsg<"batchUpdateOrders">;

export type BatchUpdateOrdersParameters = Prettify<{
  sender: Address;
  funds?: Coins;
  creates?: ActionMsg["batchUpdateOrders"]["creates"];
  cancels?: ActionMsg["batchUpdateOrders"]["cancels"];
}>;

export type BatchUpdateOrdersReturnType = BroadcastTxSyncReturnType;

export async function batchUpdateOrders<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: BatchUpdateOrdersParameters,
): BatchUpdateOrdersReturnType {
  const { creates = [], cancels, funds, sender } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    batchUpdateOrders: {
      creates,
      cancels,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "batch_update_orders", type: "BatchUpdateOrders" }],
    extraTypes: {
      BatchUpdateOrders: [
        { name: "creates", type: "CreateOrder[]" },
        ...(cancels
          ? [
              cancels === "all"
                ? { name: "cancels", type: "string" }
                : { name: "cancels", type: "CancelSome" },
            ]
          : []),
      ],
      CreateOrder: [
        { name: "base_denom", type: "string" },
        { name: "quote_denom", type: "string" },
        { name: "direction", type: "string" },
        { name: "amount", type: "AmountOption" },
        { name: "price", type: "PriceOption" },
        { name: "time_in_force", type: "string" },
      ],
      AmountOption: [
        { name: "bid", type: "Bid" },
        { name: "ask", type: "Ask" },
      ],
      Bid: [{ name: "quote", type: "string" }],
      Ask: [{ name: "base", type: "string" }],
      PriceOption: [
        { name: "limit", type: "string" },
        { name: "market", type: "Market" },
      ],
      Market: [{ name: "max_slippage", type: "string" }],
      ...(cancels && cancels !== "all" ? { CancelSome: [{ name: "some", type: "string[]" }] } : {}),
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg: msg as Json,
      typedData,
      contract: addresses.dex,
      funds,
    },
  });
}
