import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, FeeRateOverride, GetPerpsQueryMsg, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

type ActionMsg = GetPerpsQueryMsg<"feeRateOverride">;

export type GetFeeRateOverrideParameters = Prettify<ActionMsg["feeRateOverride"]>;

export type GetFeeRateOverrideReturnType = Promise<FeeRateOverride | null>;

export async function getFeeRateOverride(
  client: Client,
  parameters: GetFeeRateOverrideParameters,
): GetFeeRateOverrideReturnType {
  const msg: ActionMsg = {
    feeRateOverride: {
      ...parameters,
    },
  };

  const { addresses } = await getAppConfig(client);

  const result = (await queryWasmSmart(client, {
    contract: addresses.perps,
    msg,
  })) as [string, string] | null;

  if (!result) return null;

  return { makerFeeRate: result[0], takerFeeRate: result[1] };
}
