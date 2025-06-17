import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "#actions/app/index.js";

import type { Address, Coins, Json, Prettify, Transport } from "@left-curve/sdk/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";
import type {
  AppConfig,
  DangoClient,
  GetDexExecuteMsg,
  Signer,
  TypedDataParameter,
} from "#types/index.js";

type ActionMsg = GetDexExecuteMsg<"batchUpdateOrders">;

export type BatchUpdateOrdersParameters = Prettify<{
  sender: Address;
  funds?: Coins;
  createsLimit?: ActionMsg["batchUpdateOrders"]["createsLimit"];
  createsMarket?: ActionMsg["batchUpdateOrders"]["createsMarket"];
  cancels?: ActionMsg["batchUpdateOrders"]["cancels"];
}>;

export type BatchUpdateOrdersReturnType = BroadcastTxSyncReturnType;

export async function batchUpdateOrders<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: BatchUpdateOrdersParameters,
): BatchUpdateOrdersReturnType {
  const { createsLimit = [], createsMarket = [], cancels, funds, sender } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    batchUpdateOrders: {
      createsLimit,
      createsMarket,
      cancels,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "batch_update_orders", type: "BatchUpdateOrders" }],
    extraTypes: {
      BatchUpdateOrders: [
        { name: "creates_market", type: "CreatesMarket[]" },
        { name: "creates_limit", type: "CreatesLimit[]" },
        ...(cancels
          ? [
              cancels === "all"
                ? { name: "cancels", type: "string" }
                : { name: "cancels", type: "CancelSome" },
            ]
          : []),
      ],
      CreatesMarket: [
        { name: "base_denom", type: "string" },
        { name: "quote_denom", type: "string" },
        { name: "direction", type: "uint8" },
        { name: "amount", type: "string" },
        { name: "max_slippage", type: "string" },
      ],
      CreatesLimit: [
        { name: "base_denom", type: "string" },
        { name: "quote_denom", type: "string" },
        { name: "direction", type: "uint8" },
        { name: "amount", type: "string" },
        { name: "price", type: "string" },
      ],
      ...(cancels && cancels !== "all" ? { CancelSome: [{ name: "some", type: "uint64[]" }] } : {}),
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
