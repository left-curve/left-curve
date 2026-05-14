import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsParam, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"param">;

export type GetPerpsParamParameters = Prettify<{ height?: number }>;

export type GetPerpsParamReturnType = Promise<PerpsParam>;

export async function getPerpsParam(
  client: Client,
  parameters?: GetPerpsParamParameters,
): GetPerpsParamReturnType {
  const { height = 0 } = parameters ?? {};

  const msg: ActionMsg = {
    param: {},
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
