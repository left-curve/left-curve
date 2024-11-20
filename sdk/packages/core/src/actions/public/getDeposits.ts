import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

import type { Address, Chain, Client, Coins, Signer, Transport } from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

export type GetDepositsParameters = {
  startAfter?: Address;
  limit?: number;
  height?: number;
};

export type GetDepositsReturnType = Promise<Record<Address, Coins>>;

/**
 * Query all deposits in the factory
 * @param parameters
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The maximum number of deposits to return.
 * @param parameters.height The height at which to get the deposit.
 * @returns A record of recipient addresses and their deposits.
 */
export async function getDeposits<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetDepositsParameters,
): GetDepositsReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg = { deposits: { startAfter, limit } };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
