import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsPairParam } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"pairParams">;

export type GetPerpsPairParamsParameters = Prettify<ActionMsg["pairParams"] & { height?: number }>;

export type GetPerpsPairParamsReturnType = Promise<Record<string, PerpsPairParam>>;

export async function getPerpsPairParams(
  client: Client,
  parameters?: GetPerpsPairParamsParameters,
): GetPerpsPairParamsReturnType {
  const { height = 0, ...queryMsg } = parameters ?? {};

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    pairParams: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
