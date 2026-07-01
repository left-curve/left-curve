import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsPairParam, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"pairParams">;

export type GetPerpsPairParamsParameters = Prettify<ActionMsg["pairParams"]>;

export type GetPerpsPairParamsReturnType = Promise<Record<string, PerpsPairParam>>;

export async function getPerpsPairParams(
  client: Client,
  parameters?: GetPerpsPairParamsParameters,
): GetPerpsPairParamsReturnType {
  const msg: ActionMsg = {
    pairParams: {
      ...(parameters ?? {}),
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
