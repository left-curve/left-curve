import { getChainInfo } from "@left-curve/dango/actions";
import { getPublicClient } from "./getPublicClient.js";

import type { BlockInfo } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type GetBlockParameters = {
  height?: number;
};

export type GetBlockReturnType = BlockInfo;

export type GetBlockErrorType = Error;

export async function getBlock<config extends Config>(
  config: config,
  parameters: GetBlockParameters = {},
): Promise<GetBlockReturnType> {
  const client = getPublicClient(config);
  const { block } = await getChainInfo(client);
  return block;
}
