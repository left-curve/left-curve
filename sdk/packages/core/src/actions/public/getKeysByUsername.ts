import type {
  Address,
  Chain,
  Client,
  Hex,
  Key,
  KeyHash,
  Signer,
  Transport,
  Username,
} from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

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

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
