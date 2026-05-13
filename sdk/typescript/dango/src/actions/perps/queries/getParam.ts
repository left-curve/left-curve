import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsParam } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"param">;

export type GetPerpsParamParameters = Prettify<{ height?: number }>;

export type GetPerpsParamReturnType = Promise<PerpsParam>;

export async function getPerpsParam(
  client: Client,
  parameters?: GetPerpsParamParameters,
): GetPerpsParamReturnType {
  const { height = 0 } = parameters ?? {};

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    param: {},
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
