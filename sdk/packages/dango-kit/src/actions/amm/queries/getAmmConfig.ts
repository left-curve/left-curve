import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import type { Chain, Client, Signer, Transport } from "@left-curve/types";
import type { AmmConfig, AmmQueryMsg, AppConfig } from "../../../types/index.js";
export type GetAmmConfigParameters = {
  height?: number;
};

export type GetAmmConfigReturnType = Promise<AmmConfig>;

/**
 * Get the AMM's global configuration.
 * @param parameters
 * @param parameters.height The height at which to query the AMM's configuration.
 * @returns The AMM's global configuration.
 */
export async function getAmmConfig<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAmmConfigParameters,
): GetAmmConfigReturnType {
  const { height = 0 } = parameters;
  const msg: AmmQueryMsg = { config: {} };

  const { addresses } = await getAppConfig<AppConfig>(client);

  return await queryWasmSmart(client, { contract: addresses.amm, msg, height });
}
