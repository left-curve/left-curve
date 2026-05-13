import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetDexQueryMsg, OrderResponse } from "../../../types/dex.js";

type ActionMsg = GetDexQueryMsg<"order">;

export type GetOrderParameters = Prettify<ActionMsg["order"] & { height?: number }>;

export type GetOrderReturnType = Promise<OrderResponse>;

/**
 * Query the details of a specific order.
 * @param parameters
 * @param parameters.orderId The ID of the order.
 * @param parameters.height The height at which to query the order
 * @returns The order details.
 */
export async function getOrder(client: Client, parameters: GetOrderParameters): GetOrderReturnType {
  const { orderId, height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    order: {
      orderId,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
