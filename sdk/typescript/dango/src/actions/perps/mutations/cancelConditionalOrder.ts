import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type {
  AppConfig,
  DangoClient,
  PerpsCancelConditionalOrderRequest,
  Signer,
  TypedDataParameter,
  TypedDataProperty,
} from "../../../types/index.js";

export type CancelConditionalOrderParameters = {
  sender: Address;
  request: PerpsCancelConditionalOrderRequest;
};

export type CancelConditionalOrderReturnType = SignAndBroadcastTxReturnType;

export async function cancelConditionalOrder<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: CancelConditionalOrderParameters,
): CancelConditionalOrderReturnType {
  const { sender, request } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    trade: {
      cancelConditionalOrder: request,
    },
  };

  const cancelConditionalOrderTypedData = ((): Record<string, TypedDataProperty[]> => {
    if (request === "all") {
      return { CancelConditionalOrder: [] };
    }
    if ("one" in request) {
      return {
        CancelConditionalOrder: [{ name: "one", type: "One" }],
        One: [
          { name: "pair_id", type: "string" },
          { name: "trigger_direction", type: "string" },
        ],
      };
    }
    return {
      CancelConditionalOrder: [{ name: "all_for_pair", type: "AllForPair" }],
      AllForPair: [{ name: "pair_id", type: "string" }],
    };
  })();

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "cancel_conditional_order", type: "CancelConditionalOrder" }],
      ...cancelConditionalOrderTypedData,
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
