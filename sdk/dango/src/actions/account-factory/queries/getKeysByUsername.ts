import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Prettify, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, Key, KeyHash, UserIndexOrName } from "../../../types/index.js";

export type GetKeysByUsernameParameters = Prettify<{
  userIndexOrName: UserIndexOrName;
  startAfter?: UserIndexOrName;
  limit?: number;
  height?: number;
}>;

export type GetKeysByUsernameReturnType = Promise<Record<KeyHash, Key>>;

/**
 * Get the keys associated with a username.
 * @param parameters
 * @param parameters.userIndexOrName The username or index of the user.
 * @param parameters.height The height at which to get the keys.
 * @returns The keys associated with the username.
 */
export async function getKeysByUsername<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetKeysByUsernameParameters,
): GetKeysByUsernameReturnType {
  const { userIndexOrName, height = 0 } = parameters;
  const user =
    "index" in userIndexOrName ? { index: userIndexOrName.index } : { name: userIndexOrName.name };
  const msg = { keysByUser: { user } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
