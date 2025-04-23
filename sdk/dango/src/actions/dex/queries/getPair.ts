import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Denom, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "#types/app.js";
import type { DexQueryMsg, PairParams } from "#types/dex.js";

export type GetPairParameters = {
  quoteDenom: Denom;
  baseDenom: Denom;
  height?: number;
};

export type GetPairReturnType = Promise<PairParams>;

/**
 * Query the parameters of a single trading pair.
 * @param parameters
 * @param parameters.quoteDenom The quote denomination of the pair.
 * @param parameters.baseDenom The base denomination of the pair.
 * @param parameters.height The height at which to query the pairs
 * @returns The prices.
 */
export async function getPair<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetPairParameters,
): GetPairReturnType {
  const { quoteDenom, baseDenom, height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: DexQueryMsg = {
    pair: {
      quoteDenom,
      baseDenom,
    },
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
