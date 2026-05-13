import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsPairState, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"pairState">;

export type GetPerpsPairStateParameters = Prettify<ActionMsg["pairState"] & { height?: number }>;

export type GetPerpsPairStateReturnType = Promise<PerpsPairState | null>;

export async function getPerpsPairState(
  client: Client,
  parameters: GetPerpsPairStateParameters,
): GetPerpsPairStateReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const msg: ActionMsg = {
    pairState: {
      ...queryMsg,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
