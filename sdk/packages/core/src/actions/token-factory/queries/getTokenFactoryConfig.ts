import type {
  Address,
  Chain,
  Client,
  Signer,
  TokenFactoryConfig,
  TokenFactoryQueryMsg,
  Transport,
} from "@leftcurve/types";
import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

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

  const contract = await getAppConfig<Address>(client, { key: "token_factory" });

  return await queryWasmSmart(client, { contract, msg, height });
}
