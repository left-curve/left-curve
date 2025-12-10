import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Address, Chain, Client, Prettify, Signer, Transport } from "@left-curve/sdk/types";
import type { AccountInfo, AppConfig, UserIndexOrName } from "../../../types/index.js";

export type GetAccountsByUsernameParameters = Prettify<{
  height?: number;
  userIndexOrName: UserIndexOrName;
}>;

export type GetAccountsByUsernameReturnType = Promise<Record<Address, AccountInfo>>;

/**
 * Find all accounts associated with a user.
 * @param parameters
 * @param parameters.userIndexOrName The username or index of the user.
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
  const { userIndexOrName, height = 0 } = parameters;
  const user =
    "index" in userIndexOrName ? { index: userIndexOrName.index } : { name: userIndexOrName.name };
  const msg = { accountsByUser: { user } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
