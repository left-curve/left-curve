import type {
  Address,
  AmmConfig,
  AmmQueryMsg,
  Chain,
  Client,
  Signer,
  Transport,
} from "@leftcurve/types";
import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

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

  const ammAddr = await getAppConfig<Address>(client, { key: "amm" });

  return await queryWasmSmart(client, { contract: ammAddr, msg, height });
}
