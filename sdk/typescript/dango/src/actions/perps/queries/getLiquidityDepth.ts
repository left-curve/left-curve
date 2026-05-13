import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsLiquidityDepthResponse } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"liquidityDepth">;

export type GetPerpsLiquidityDepthParameters = Prettify<
  ActionMsg["liquidityDepth"] & { height?: number }
>;

export type GetPerpsLiquidityDepthReturnType = Promise<PerpsLiquidityDepthResponse>;

export async function getPerpsLiquidityDepth(
  client: Client,
  parameters: GetPerpsLiquidityDepthParameters,
): GetPerpsLiquidityDepthReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    liquidityDepth: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
