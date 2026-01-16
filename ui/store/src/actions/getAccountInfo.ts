import {
  getAccountInfo as getAccountInfoSdkAction,
  getUsernameByIndex,
} from "@left-curve/dango/actions";
import { getPublicClient } from "./getPublicClient.js";

import type { Account, AccountInfo, AccountTypes } from "@left-curve/dango/types";
import type { Address, ChainId } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";

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

  const { params } = account as AccountInfo<"single">;

  const username = ["margin", "single"].includes(type)
    ? await getUsernameByIndex(client, { index: params.single.owner })
    : "Multisig";

  return {
    ...account,
    username: username ? username : `User #${params.single.owner}`,
    type,
    address: parameters.address,
  };
}
