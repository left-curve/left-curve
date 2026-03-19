import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type {
  AppConfig,
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
};

export type SubmitPerpsOrderReturnType = SignAndBroadcastTxReturnType;

export async function submitPerpsOrder<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SubmitPerpsOrderParameters,
): SubmitPerpsOrderReturnType {
  const { sender, pairId, size, kind, reduceOnly } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    trade: {
      submitOrder: {
        pairId,
        size,
        kind,
        reduceOnly,
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

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "submitOrder", type: "SubmitOrder" }],
      SubmitOrder: [
        { name: "pairId", type: "string" },
        { name: "size", type: "string" },
        { name: "kind", type: "Kind" },
        { name: "reduceOnly", type: "bool" },
      ],
      Kind: kindTypedData.kind,
      ...(kindTypedData.Market ? { Market: kindTypedData.Market } : {}),
      ...(kindTypedData.Limit ? { Limit: kindTypedData.Limit } : {}),
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
