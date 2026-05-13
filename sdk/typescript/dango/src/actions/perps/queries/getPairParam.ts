import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsPairParam } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"pairParam">;

export type GetPerpsPairParamParameters = Prettify<ActionMsg["pairParam"] & { height?: number }>;

export type GetPerpsPairParamReturnType = Promise<PerpsPairParam | null>;

export async function getPerpsPairParam(
  client: Client,
  parameters: GetPerpsPairParamParameters,
): GetPerpsPairParamReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    pairParam: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
