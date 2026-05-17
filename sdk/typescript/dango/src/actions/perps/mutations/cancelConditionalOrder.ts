import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address } from "@left-curve/types";
import type { SignAndBroadcastTxReturnType } from "#actions/app/mutations/signAndBroadcastTx.js";
import type {
  Client,
  PerpsCancelConditionalOrderRequest,
  Signer,
  TypedDataParameter,
  TypedDataProperty,
} from "@left-curve/types";

export type CancelConditionalOrderParameters = {
  sender: Address;
  request: PerpsCancelConditionalOrderRequest;
};

export type CancelConditionalOrderReturnType = SignAndBroadcastTxReturnType;

export async function cancelConditionalOrder(
  client: Client<Signer>,
  parameters: CancelConditionalOrderParameters,
): CancelConditionalOrderReturnType {
  const { sender, request } = parameters;

  const { addresses } = await getAppConfig(client);

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
