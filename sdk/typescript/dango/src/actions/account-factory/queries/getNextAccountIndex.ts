import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { AccountIndex, Client, Username } from "@left-curve/types";

export type GetNextAccountIndexParameters = {
  username: Username;
  height?: number;
};

export type GetNextAccountIndexReturnType = Promise<AccountIndex>;

/**
 * Query the account index, which is used in deriving the account address,
 * must be used if a user is to create a new account.
 * @param parameters
 * @param parameters.username The username referece to get the next index.
 * @param parameters.height The height at which to get the accounts.
 * @returns The index.
 */
export async function getNextAccountIndex(
  client: Client,
  parameters: GetNextAccountIndexParameters,
): GetNextAccountIndexReturnType {
  const { username, height = 0 } = parameters;
  const msg = { nextAccountIndex: { username } };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
