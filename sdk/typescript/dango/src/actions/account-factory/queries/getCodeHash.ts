import { getAppConfig, queryWasmSmart } from "../../../index.js";
import { getAction } from "../../index.js";
import type { Client, Hex } from "../../../types/index.js";
import type { AppConfig } from "../../../types/index.js";

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
  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
