import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, User, Username } from "../../../types/index.js";

export type GetUserParameters = {
  username: Username;
  height?: number;
};

export type GetUserReturnType = Promise<User>;

/**
 * Given a username get the user.
 * @param parameters
 * @param parameters.username The username of the user.
 * @param parameters.height The height at which to get the user.
 * @returns The user
 */
export async function getUser<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetUserParameters,
): GetUserReturnType {
  const { username, height = 0 } = parameters;
  const msg = { user: { username } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
