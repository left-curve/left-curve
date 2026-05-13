import { getAppConfig, queryWasmSmart } from "../../../index.js";
import { getAction } from "../../index.js";
import type { Address, Client } from "../../../types/index.js";
import type { AccountInfo, AppConfig } from "../../../types/index.js";

export type GetAllAccountInfoParameters = {
  startAfter?: Address;
  limit?: number;
  height?: number;
};

export type GetAllAccountInfoReturnType = Promise<Record<Address, AccountInfo>>;

/**
 * Get all account info in the factory.
 * @param parameters
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The maximum number of accounts to return.
 * @param parameters.height The height at which to get all account info.
 * @returns A record of address and account info.
 */
export async function getAllAccountInfo(
  client: Client,
  parameters: GetAllAccountInfoParameters,
): GetAllAccountInfoReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg = { accounts: { startAfter, limit } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
