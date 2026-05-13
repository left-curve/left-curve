import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsLiquidityDepthResponse } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"liquidityDepth">;

export type GetPerpsLiquidityDepthParameters = Prettify<
  ActionMsg["liquidityDepth"] & { height?: number }
>;

export type GetPerpsLiquidityDepthReturnType = Promise<PerpsLiquidityDepthResponse>;

export async function getPerpsLiquidityDepth<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
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
