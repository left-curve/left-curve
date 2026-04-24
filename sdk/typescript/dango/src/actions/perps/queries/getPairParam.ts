import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsPairParam } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"pairParam">;

export type GetPerpsPairParamParameters = Prettify<ActionMsg["pairParam"] & { height?: number }>;

export type GetPerpsPairParamReturnType = Promise<PerpsPairParam | null>;

export async function getPerpsPairParam<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
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
