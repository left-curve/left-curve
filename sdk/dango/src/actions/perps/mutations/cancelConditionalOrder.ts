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

  const cancelConditionalOrderTypedData = (() => {
    if (request === "all") {
      return { CancelConditionalOrder: [] };
    }
    if ("one" in request) {
      return {
        CancelConditionalOrder: [{ name: "one", type: "One" }],
        One: [
          { name: "pairId", type: "string" },
          { name: "triggerDirection", type: "string" },
        ],
      };
    }
    return {
      CancelConditionalOrder: [{ name: "allForPair", type: "AllForPair" }],
      AllForPair: [{ name: "pairId", type: "string" }],
    };
  })();

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "cancelConditionalOrder", type: "CancelConditionalOrder" }],
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
