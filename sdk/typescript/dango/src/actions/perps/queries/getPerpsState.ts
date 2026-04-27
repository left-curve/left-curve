import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsState } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"state">;

export type GetPerpsStateParameters = Prettify<{ height?: number }>;

export type GetPerpsStateReturnType = Promise<PerpsState>;

export async function getPerpsState<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters?: GetPerpsStateParameters,
): GetPerpsStateReturnType {
  const { height = 0 } = parameters ?? {};

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    state: {},
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.perps, msg, height });
}
