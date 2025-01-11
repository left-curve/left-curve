import { getAccountInfo as getAccountInfoSdkAction } from "@left-curve/sdk/actions";
import { getPublicClient } from "./getPublicClient.js";

import type { Account, AccountTypes, Address, ChainId, Config } from "@left-curve/types";

export type GetAccountInfoParameters = {
  address: Address;
  chainId?: ChainId;
  height?: number;
};

export type GetAccountInfoReturnType = Account | null;

export type GetAccountInfoErrorType = Error;

export async function getAccountInfo<config extends Config>(
  config: config,
  parameters: GetAccountInfoParameters,
): Promise<GetAccountInfoReturnType> {
  const client = getPublicClient(config);
  const account = await getAccountInfoSdkAction(client, parameters);

  if (!account) return null;

  const type = Object.keys(account.params).at(0) as AccountTypes;

  const username = ["margin", "spot"].includes(type)
    ? (account.params as { [key: string]: { owner: string } })[type].owner
    : "";

  return {
    ...account,
    username,
    type,
    address: parameters.address,
  };
}
