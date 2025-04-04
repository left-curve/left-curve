import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Address, Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AccountInfo, AppConfig, Username } from "../../../types/index.js";

export type GetAccountsByUsernameParameters = {
  username: Username;
  height?: number;
};

export type GetAccountsByUsernameReturnType = Promise<Record<Address, AccountInfo>>;

/**
 * Find all accounts associated with a user.
 * @param parameters
 * @param parameters.username The username to get accounts for.
 * @param parameters.height The height at which to get the accounts.
 * @returns The accounts.
 */
export async function getAccountsByUsername<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountsByUsernameParameters,
): GetAccountsByUsernameReturnType {
  const { username, height = 0 } = parameters;
  const msg = { accountsByUser: { username } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
