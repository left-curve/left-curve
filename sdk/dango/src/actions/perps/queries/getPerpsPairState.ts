import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsPairState } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"pairState">;

export type GetPerpsPairStateParameters = Prettify<ActionMsg["pairState"] & { height?: number }>;

export type GetPerpsPairStateReturnType = Promise<PerpsPairState | null>;

export async function getPerpsPairState<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetPerpsPairStateParameters,
): GetPerpsPairStateReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    pairState: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
