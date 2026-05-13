import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { FeeRateOverride, GetPerpsQueryMsg } from "../../../types/perps.js";

type ActionMsg = GetPerpsQueryMsg<"feeRateOverride">;

export type GetFeeRateOverrideParameters = Prettify<
  ActionMsg["feeRateOverride"] & { height?: number }
>;

export type GetFeeRateOverrideReturnType = Promise<FeeRateOverride | null>;

export async function getFeeRateOverride(
  client: Client,
  parameters: GetFeeRateOverrideParameters,
): GetFeeRateOverrideReturnType {
  const { height = 0, ...queryMsg } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: ActionMsg = {
    feeRateOverride: {
      ...queryMsg,
    },
  };

  const { addresses } = await action<AppConfig>({});

  const result = (await queryWasmSmart(client, {
    contract: addresses.perps,
    msg,
    height,
  })) as [string, string] | null;

  if (!result) return null;

  return { makerFeeRate: result[0], takerFeeRate: result[1] };
}
