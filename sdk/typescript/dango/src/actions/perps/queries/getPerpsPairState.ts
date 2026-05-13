import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsPairState } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"pairState">;

export type GetPerpsPairStateParameters = Prettify<ActionMsg["pairState"] & { height?: number }>;

export type GetPerpsPairStateReturnType = Promise<PerpsPairState | null>;

export async function getPerpsPairState(
  client: Client,
  parameters: GetPerpsPairStateParameters,
): GetPerpsPairStateReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    pairState: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
