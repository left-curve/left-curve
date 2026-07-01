import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, GetPerpsQueryMsg, PerpsUserState, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"userState">;

export type GetPerpsUserStateParameters = Prettify<ActionMsg["userState"]>;

export type GetPerpsUserStateReturnType = Promise<PerpsUserState | null>;

export async function getPerpsUserState(
  client: Client,
  parameters: GetPerpsUserStateParameters,
): GetPerpsUserStateReturnType {
  const msg: ActionMsg = {
    userState: {
      ...parameters,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
