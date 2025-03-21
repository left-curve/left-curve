import {
  type GetBalancesParameters as _GetBalancesParameters_,
  getBalances as getBalancesAction,
} from "@left-curve/dango/actions";
import { getPublicClient } from "./getPublicClient.js";

import type { ChainId, Coins, Prettify } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type GetBalanceParameters = Prettify<
  _GetBalancesParameters_ & {
    chainId?: ChainId;
  }
>;

export type GetBalancesReturnType = Coins;

export type GetBalancesErrorType = Error;

export async function getBalances<config extends Config>(
  config: config,
  parameters: GetBalanceParameters,
): Promise<GetBalancesReturnType> {
  const client = getPublicClient(config, parameters);
  return await getBalancesAction(client, parameters);
}
