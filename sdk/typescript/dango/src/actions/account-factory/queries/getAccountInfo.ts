import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import { getUser } from "./getUser.js";

import type { AccountDetails, AccountInfo, Address, Client } from "@left-curve/types";

export type GetAccountInfoParameters = {
  address: Address;
};

export type GetAccountInfoReturnType = Promise<AccountDetails | null>;

/**
 * Given an account address get the account info.
 * @param parameters
 * @param parameters.address The address of the account.
 * @returns The account info.
 */
export async function getAccountInfo(
  client: Client,
  parameters: GetAccountInfoParameters,
): GetAccountInfoReturnType {
  const { address } = parameters;
  const msg = { account: { address } };

  const { addresses } = await getAppConfig(client);

  const account = await queryWasmSmart<AccountInfo>(client, {
    contract: addresses.accountFactory,
    msg,
  });

  if (!account) return null;

  const user = await getUser(client, { userIndexOrName: { index: account.owner } });

  return {
    ...account,
    username: user.name,
    address: parameters.address,
  };
}
