import { getAppConfig, queryWasmSmart } from "../../../index.js";
import type { AccountIndex, AppConfig, Username } from "../../../types/index.js";

import { getAction } from "../../index.js";
import type { Client } from "../../../types/index.js";

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

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
