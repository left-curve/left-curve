import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "#types/app.js";
import type { DexQueryMsg, PairId, PairUpdate } from "#types/dex.js";

export type GetPairsParameters = {
  limit?: number;
  startAfter?: PairId;
  height?: number;
};

export type GetPairsReturnType = Promise<PairUpdate[]>;

/**
 * Enumerate all trading pairs and their parameters.
 * @param parameters
 * @param parameters.limit The maximum number of pairs to return.
 * @param parameters.startAfter The ID of the pair to start after.
 * @param parameters.height The height at which to query the pairs
 * @returns The pairs and their parameters.
 */
export async function getPairs<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetPairsParameters = {},
): GetPairsReturnType {
  const { limit, startAfter, height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: DexQueryMsg = {
    pairs: {
      limit,
      startAfter,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
