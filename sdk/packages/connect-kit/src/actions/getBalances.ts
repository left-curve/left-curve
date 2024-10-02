import {
  type GetBalancesParameters as _GetBalancesParameters_,
  getBalances as getBalancesAction,
} from "@leftcurve/sdk/actions";
import { getPublicClient } from "./getPublicClient";

import type { ChainId, Coins, Config, Prettify } from "@leftcurve/types";

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
