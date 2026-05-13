import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsState, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"state">;

export type GetPerpsStateParameters = Prettify<{ height?: number }>;

export type GetPerpsStateReturnType = Promise<PerpsState>;

export async function getPerpsState(
  client: Client,
  parameters?: GetPerpsStateParameters,
): GetPerpsStateReturnType {
  const { height = 0 } = parameters ?? {};

  const msg: ActionMsg = {
    state: {},
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
