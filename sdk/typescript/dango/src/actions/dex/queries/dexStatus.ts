import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, DexQueryMsg } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type DexStatusParameters = {
  height?: number;
};

export type DexStatusReturnType = Promise<boolean>;

/**
 * Queries the DEX status, whether it is paused.
 * @param parameters
 * @param parameters.height The height at which to query the dex status.
 * @returns The DEX status.
 */
export async function dexStatus(
  client: Client,
  parameters: DexStatusParameters = {},
): DexStatusReturnType {
  const { height = 0 } = parameters;

  const msg: DexQueryMsg = {
    paused: {},
  };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
