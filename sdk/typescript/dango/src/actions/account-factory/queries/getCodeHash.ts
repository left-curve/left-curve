import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Hex } from "@left-curve/types";

export type GetCodeHashParameters = {
  height?: number;
};

export type GetCodeHashReturnType = Promise<Hex>;

/**
 * Get the account code hash.
 * @param parameters
 * @param parameters.height The height at which to query the code hash.
 * @returns The account code hash.
 */
export async function getCodeHash(
  client: Client,
  parameters?: GetCodeHashParameters,
): GetCodeHashReturnType {
  const { height = 0 } = parameters || {};
  const msg = { codeHash: {} };
  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
