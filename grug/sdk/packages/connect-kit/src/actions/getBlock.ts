import { getChainInfo } from "@leftcurve/sdk/actions";
import { getPublicClient } from "./getPublicClient";

import type { BlockInfo, ChainId, Config } from "@leftcurve/types";

export type GetBlockParameters = {
  chainId?: ChainId;
  height?: number;
};

export type GetBlockReturnType = BlockInfo;

export type GetBlockErrorType = Error;

export async function getBlock<config extends Config>(
  config: config,
  parameters: GetBlockParameters = {},
): Promise<GetBlockReturnType> {
  const client = getPublicClient(config, parameters);
  const { lastFinalizedBlock } = await getChainInfo(client, parameters);
  return lastFinalizedBlock;
}
