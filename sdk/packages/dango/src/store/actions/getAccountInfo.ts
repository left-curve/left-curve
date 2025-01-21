import { getAccountInfo as getAccountInfoSdkAction } from "../../actions/account-factory/queries/getAccountInfo.js";
import { getPublicClient } from "./getPublicClient.js";

import type { Address, ChainId } from "@left-curve/types";
import type { Account, AccountTypes, Config } from "../../types/index.js";

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
