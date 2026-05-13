import { getAppConfig, queryWasmSmart } from "../../../index.js";

import { getAction } from "../../index.js";
import type { Client } from "../../../types/index.js";
import type { AppConfig, KeyHash, User } from "../../../types/index.js";

export type ForgotUsernameParameters = {
  keyHash: KeyHash;
  limit?: number;
  startAfter?: number;
  height?: number;
};

export type ForgotUsernameReturnType = Promise<User[]>;
/**
 * Given a key hash, get the user(s) associated with it.
 * @param parameters
 * @param parameters.keyHash The key hash to get the user for.
 * @param parameters.limit The maximum number of users to return.
 * @param parameters.startAfter The user index to start after.
 * @param parameters.height The height at which query is made.
 * @returns The user(s)
 */
export async function forgotUsername(
  client: Client,
  parameters: ForgotUsernameParameters,
): ForgotUsernameReturnType {
  const { keyHash, limit, startAfter, height = 0 } = parameters;
  const msg = { forgotUsername: { keyHash, limit, startAfter } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, {
    contract: addresses.accountFactory,
    msg,
    height,
  });
}
