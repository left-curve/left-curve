import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";

import type { Client, User, UserIndexOrName } from "@left-curve/types";

export type GetUserParameters = {
  userIndexOrName: UserIndexOrName;
  height?: number;
};

export type GetUserReturnType = Promise<User>;

/**
 * Given a user index or name, get the user.
 * @param parameters
 * @param parameters.userIndexOrName The index or name of the user.
 * @param parameters.height The height at which to get the user.
 * @returns The user
 */
export async function getUser(client: Client, parameters: GetUserParameters): GetUserReturnType {
  const { userIndexOrName, height = 0 } = parameters;
  const msg = { user: userIndexOrName };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
