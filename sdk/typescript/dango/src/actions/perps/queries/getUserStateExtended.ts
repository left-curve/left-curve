import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsUserStateExtended } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"userStateExtended">;

export type GetPerpsUserStateExtendedParameters = Prettify<
  ActionMsg["userStateExtended"] & { height?: number }
>;

export type GetPerpsUserStateExtendedReturnType = Promise<PerpsUserStateExtended | null>;

export async function getPerpsUserStateExtended(
  client: Client,
  parameters: GetPerpsUserStateExtendedParameters,
): GetPerpsUserStateExtendedReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    userStateExtended: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
