import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Hex } from "@left-curve/types";

export type GetCodeHashReturnType = Promise<Hex>;

/**
 * Get the account code hash.
 * @returns The account code hash.
 */
export async function getCodeHash(client: Client): GetCodeHashReturnType {
  const msg = { codeHash: {} };
  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg });
}
