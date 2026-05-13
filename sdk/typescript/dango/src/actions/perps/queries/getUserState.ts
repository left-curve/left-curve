import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsUserState } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"userState">;

export type GetPerpsUserStateParameters = Prettify<ActionMsg["userState"] & { height?: number }>;

export type GetPerpsUserStateReturnType = Promise<PerpsUserState | null>;

export async function getPerpsUserState(
  client: Client,
  parameters: GetPerpsUserStateParameters,
): GetPerpsUserStateReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    userState: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
