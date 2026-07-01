import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsParam } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"param">;

export type GetPerpsParamReturnType = Promise<PerpsParam>;

export async function getPerpsParam(client: Client): GetPerpsParamReturnType {
  const msg: ActionMsg = {
    param: {},
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
