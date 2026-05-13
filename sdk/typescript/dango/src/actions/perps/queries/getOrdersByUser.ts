import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsOrdersByUserResponse } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"ordersByUser">;

export type GetPerpsOrdersByUserParameters = Prettify<
  ActionMsg["ordersByUser"] & { height?: number }
>;

export type GetPerpsOrdersByUserReturnType = Promise<PerpsOrdersByUserResponse>;

export async function getPerpsOrdersByUser(
  client: Client,
  parameters: GetPerpsOrdersByUserParameters,
): GetPerpsOrdersByUserReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    ordersByUser: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
