import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { AccountInfo, Address, Client } from "@left-curve/types";

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

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
