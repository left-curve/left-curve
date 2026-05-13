import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetDexQueryMsg, OrderResponse, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

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

  const msg: ActionMsg = {
    order: {
      orderId,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
