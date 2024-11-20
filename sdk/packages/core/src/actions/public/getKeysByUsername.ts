import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

import type {
  Chain,
  Client,
  Hex,
  Key,
  KeyHash,
  Signer,
  Transport,
  Username,
} from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

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

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
