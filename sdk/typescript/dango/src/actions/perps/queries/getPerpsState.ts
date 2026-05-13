import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsState } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"state">;

export type GetPerpsStateParameters = Prettify<{ height?: number }>;

export type GetPerpsStateReturnType = Promise<PerpsState>;

export async function getPerpsState(
  client: Client,
  parameters?: GetPerpsStateParameters,
): GetPerpsStateReturnType {
  const { height = 0 } = parameters ?? {};

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    state: {},
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
