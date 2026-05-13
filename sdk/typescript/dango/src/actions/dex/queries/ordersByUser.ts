import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type {
  Client,
  GetDexQueryMsg,
  OrderId,
  OrdersByUserResponse,
  Prettify,
} from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetDexQueryMsg<"ordersByUser">;

export type OrdersByUserParameters = Prettify<ActionMsg["ordersByUser"] & { height?: number }>;

export type OrdersByUserReturnType = Promise<Record<OrderId, OrdersByUserResponse>>;

/**
 * Query orders by user.
 * This function retrieves orders placed by a specific user on the Dango DEX.
 * @param parameters
 * @param parameters.user The user address to query orders for.
 * @param parameters.startAfter The ID of the order to start after.
 * @param parameters.limit The maximum number of orders to return.
 * @param parameters.height The height at which to query the pairs
 * @returns The orders by user response.
 */
export async function ordersByUser(
  client: Client,
  parameters: OrdersByUserParameters,
): OrdersByUserReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const msg: ActionMsg = {
    ordersByUser: {
      ...queryMsg,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
