import { getAction, getAppConfig, queryWasmSmart } from "../../index.js";
import { getUser } from "./getUser.js";

import type { Address, Client } from "../../../types/index.js";
import type { AccountDetails, AccountInfo, AppConfig } from "../../../types/index.js";

export type GetAccountInfoParameters = {
  address: Address;
  height?: number;
};

export type GetAccountInfoReturnType = Promise<AccountDetails | null>;

/**
 * Given an account address get the account info.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to get the account info.
 * @returns The account info.
 */
export async function getAccountInfo(
  client: Client,
  parameters: GetAccountInfoParameters,
): GetAccountInfoReturnType {
  const { address, height = 0 } = parameters;
  const msg = { account: { address } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  const account = await queryWasmSmart<AccountInfo>(client, {
    contract: addresses.accountFactory,
    msg,
    height,
  });

  if (!account) return null;

  const user = await getUser(client, { userIndexOrName: { index: account.owner }, height });

  return {
    ...account,
    username: user.name,
    address: parameters.address,
  };
}
