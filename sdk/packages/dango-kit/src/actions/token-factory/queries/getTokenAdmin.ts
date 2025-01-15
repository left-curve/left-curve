import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import type { AppConfig, TokenFactoryQueryMsg } from "../../../types/index.js";

import type { Address, Chain, Client, Denom, Signer, Transport } from "@left-curve/types";

export type GetTokenAdminParameters = {
  denom: Denom;
  height?: number;
};

export type GetTokenAdminReturnType = Promise<Address>;

/**
 * Get the admin address of a denom.
 * @param parameters
 * @param parameters.denom The denom to query the admin address of.
 * @param parameters.height The height to query the admin address of the denom.
 * @returns the admin address of the denom.
 */
export async function getTokenAdmin<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetTokenAdminParameters,
): GetTokenAdminReturnType {
  const { denom, height = 0 } = parameters;
  const msg: TokenFactoryQueryMsg = { admin: { denom } };

  const { addresses } = await getAppConfig<AppConfig>(client);

  return await queryWasmSmart(client, { contract: addresses.tokenFactory, msg, height });
}
