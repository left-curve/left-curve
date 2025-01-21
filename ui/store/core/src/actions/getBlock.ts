import { getChainInfo } from "@left-curve/dango/actions";
import { getPublicClient } from "./getPublicClient.js";

import type { BlockInfo, ChainId } from "@left-curve/types";
import type { Config } from "../types/store.js";

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
  const { lastFinalizedBlock } = await getChainInfo(client);
  return lastFinalizedBlock;
}
