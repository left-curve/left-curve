import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type {
  Client,
  GetPerpsQueryMsg,
  PerpsLiquidityDepthResponse,
  Prettify,
} from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"liquidityDepth">;

export type GetPerpsLiquidityDepthParameters = Prettify<ActionMsg["liquidityDepth"]>;

export type GetPerpsLiquidityDepthReturnType = Promise<PerpsLiquidityDepthResponse>;

export async function getPerpsLiquidityDepth(
  client: Client,
  parameters: GetPerpsLiquidityDepthParameters,
): GetPerpsLiquidityDepthReturnType {
  const msg: ActionMsg = {
    liquidityDepth: {
      ...parameters,
    },
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.perps, msg });
}
