import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsPairParam, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"pairParam">;

export type GetPerpsPairParamParameters = Prettify<ActionMsg["pairParam"]>;

export type GetPerpsPairParamReturnType = Promise<PerpsPairParam | null>;

export async function getPerpsPairParam(
  client: Client,
  parameters: GetPerpsPairParamParameters,
): GetPerpsPairParamReturnType {
  const msg: ActionMsg = {
    pairParam: {
      ...parameters,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
