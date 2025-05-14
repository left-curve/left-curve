import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, KeyHash, Username } from "../../../types/index.js";

export type ForgotUsernameParameters = {
  keyHash: KeyHash;
  limit?: number;
  startAfter?: Username;
  height?: number;
};

export type ForgotUsernameReturnType = Promise<Username[]>;
/**
 * Given a key hash, get the username(s) associated with it.
 * @param parameters
 * @param parameters.keyHash The key hash to get the username for.
 * @param parameters.limit The maximum number of usernames to return.
 * @param parameters.startAfter The username to start after.
 * @param parameters.height The height at which query is made.
 * @returns The username(s)
 */
export async function forgotUsername<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: ForgotUsernameParameters,
): ForgotUsernameReturnType {
  const { keyHash, limit, startAfter, height = 0 } = parameters;
  const msg = { forgotUsername: { keyHash, limit, startAfter } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
