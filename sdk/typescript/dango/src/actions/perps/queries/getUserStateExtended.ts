import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsUserStateExtended, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

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

  const msg: ActionMsg = {
    userStateExtended: {
      ...queryMsg,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
