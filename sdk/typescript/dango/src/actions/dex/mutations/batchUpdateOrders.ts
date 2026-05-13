import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type {
  Address,
  Client,
  Coins,
  GetDexExecuteMsg,
  Json,
  Prettify,
  Signer,
  TypedDataParameter,
} from "@left-curve/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";

type ActionMsg = GetDexExecuteMsg<"batchUpdateOrders">;

export type BatchUpdateOrdersParameters = Prettify<{
  sender: Address;
  funds?: Coins;
  creates?: ActionMsg["batchUpdateOrders"]["creates"];
  cancels?: ActionMsg["batchUpdateOrders"]["cancels"];
}>;

export type BatchUpdateOrdersReturnType = BroadcastTxSyncReturnType;

export async function batchUpdateOrders(
  client: Client<Signer>,
  parameters: BatchUpdateOrdersParameters,
): BatchUpdateOrdersReturnType {
  const { creates = [], cancels, funds, sender } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg = {
    batchUpdateOrders: {
      creates,
      cancels,
    },
  };

  const [order] = creates;
  const isLimit = Object.hasOwn(order?.price || {}, "limit");
  const isBuy = Object.hasOwn(order?.amount || {}, "bid");

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
        { name: "amount", type: "AmountOption" },
        { name: "price", type: "PriceOption" },
        { name: "time_in_force", type: "string" },
      ],
      AmountOption: [isBuy ? { name: "bid", type: "Bid" } : { name: "ask", type: "Ask" }],
      ...(isBuy
        ? { Bid: [{ name: "quote", type: "string" }] }
        : { Ask: [{ name: "base", type: "string" }] }),
      PriceOption: [
        isLimit ? { name: "limit", type: "string" } : { name: "market", type: "Market" },
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
