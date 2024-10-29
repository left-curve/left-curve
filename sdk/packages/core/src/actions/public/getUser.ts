import type { Address, Chain, Client, Signer, Transport, User, Username } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

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

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
