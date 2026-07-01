import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type {
  Client,
  GetPerpsQueryMsg,
  PerpsOrdersByUserResponse,
  Prettify,
} from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"ordersByUser">;

export type GetPerpsOrdersByUserParameters = Prettify<ActionMsg["ordersByUser"]>;

export type GetPerpsOrdersByUserReturnType = Promise<PerpsOrdersByUserResponse>;

export async function getPerpsOrdersByUser(
  client: Client,
  parameters: GetPerpsOrdersByUserParameters,
): GetPerpsOrdersByUserReturnType {
  const msg: ActionMsg = {
    ordersByUser: {
      ...parameters,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
