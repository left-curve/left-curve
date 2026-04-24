import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { GetPerpsQueryMsg, PerpsUserStateExtended } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"userStateExtended">;

export type GetPerpsUserStateExtendedParameters = Prettify<
  ActionMsg["userStateExtended"] & { height?: number }
>;

export type GetPerpsUserStateExtendedReturnType = Promise<PerpsUserStateExtended | null>;

export async function getPerpsUserStateExtended<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
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
