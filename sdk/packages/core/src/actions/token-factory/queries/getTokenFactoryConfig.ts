import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

import type {
  Chain,
  Client,
  Signer,
  TokenFactoryConfig,
  TokenFactoryQueryMsg,
  Transport,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type GetTokenFactoryConfigParameters = {
  height?: number;
};

export type GetTokenFactoryConfigReturnType = Promise<TokenFactoryConfig>;

/**
 * Get the TokenFactory's global configuration.
 * @param parameters
 * @param parameters.height The height at which to query the TokenFactory's configuration.
 * @returns the TokenFactory's global configuration.
 */
export async function getTokenFactoryConfig<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetTokenFactoryConfigParameters,
): GetTokenFactoryConfigReturnType {
  const { height = 0 } = parameters;
  const msg: TokenFactoryQueryMsg = { config: {} };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.tokenFactory, msg, height });
}
