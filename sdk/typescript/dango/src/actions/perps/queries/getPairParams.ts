import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsPairParam } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"pairParams">;

export type GetPerpsPairParamsParameters = Prettify<ActionMsg["pairParams"] & { height?: number }>;

export type GetPerpsPairParamsReturnType = Promise<Record<string, PerpsPairParam>>;

export async function getPerpsPairParams<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters?: GetPerpsPairParamsParameters,
): GetPerpsPairParamsReturnType {
  const { height = 0, ...queryMsg } = parameters ?? {};

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    pairParams: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
