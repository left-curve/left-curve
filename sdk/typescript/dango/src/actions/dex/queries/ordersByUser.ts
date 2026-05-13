import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetDexQueryMsg, OrderId, OrdersByUserResponse } from "../../../types/dex.js";

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

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    ordersByUser: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
