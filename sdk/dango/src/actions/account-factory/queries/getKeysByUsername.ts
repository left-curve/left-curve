import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Hex, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, Key, KeyHash, Username } from "../../../types/index.js";

export type GetKeysByUsernameParameters = {
  username: Username;
  startAfter?: Hex;
  limit?: number;
  height?: number;
};

export type GetKeysByUsernameReturnType = Promise<Record<KeyHash, Key>>;

/**
 * Get the keys associated with a username.
 * @param parameters
 * @param parameters.username The username to get keys for.
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
  const { username, height = 0 } = parameters;
  const msg = { keysByUser: { username } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
