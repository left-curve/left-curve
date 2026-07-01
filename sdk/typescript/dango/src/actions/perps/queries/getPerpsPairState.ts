import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsPairState, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"pairState">;

export type GetPerpsPairStateParameters = Prettify<ActionMsg["pairState"]>;

export type GetPerpsPairStateReturnType = Promise<PerpsPairState | null>;

export async function getPerpsPairState(
  client: Client,
  parameters: GetPerpsPairStateParameters,
): GetPerpsPairStateReturnType {
  const msg: ActionMsg = {
    pairState: {
      ...parameters,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
