import { getAccountInfo as getAccountInfoSdkAction } from "@left-curve/dango/actions";
import { getPublicClient } from "./getPublicClient.js";

import type { Account } from "@left-curve/dango/types";
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
  return await getAccountInfoSdkAction(client, parameters);
}
