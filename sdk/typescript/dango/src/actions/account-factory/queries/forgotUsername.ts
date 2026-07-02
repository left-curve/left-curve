import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";

import type { Client, KeyHash, User } from "@left-curve/types";

export type ForgotUsernameParameters = {
  keyHash: KeyHash;
  limit?: number;
  startAfter?: number;
};

export type ForgotUsernameReturnType = Promise<User[]>;
/**
 * Given a key hash, get the user(s) associated with it.
 * @param parameters
 * @param parameters.keyHash The key hash to get the user for.
 * @param parameters.limit The maximum number of users to return.
 * @param parameters.startAfter The user index to start after.
 * @returns The user(s)
 */
export async function forgotUsername(
  client: Client,
  parameters: ForgotUsernameParameters,
): ForgotUsernameReturnType {
  const { keyHash, limit, startAfter } = parameters;
  const msg = { forgotUsername: { keyHash, limit, startAfter } };

  const { addresses } = await getAppConfig(client);

  return await queryWasmSmart(client, {
    contract: addresses.accountFactory,
    msg,
  });
}
