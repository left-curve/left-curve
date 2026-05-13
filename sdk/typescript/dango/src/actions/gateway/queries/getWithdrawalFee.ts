import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Denom, Remote } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type GetWithdrawalFeeParameters = {
  denom: Denom;
  remote: Remote;
  height?: number;
};

export type GetWithdrawalFeeReturnType = Promise<string>;

/**
 * Query the withdrawal fee for a given denom and remote.
 * @param parameters
 * @param parameters.denom The denomination to query.
 * @param parameters.remote The remote chain to query.
 * @param parameters.height The height at which to query the withdrawal fee.
 * @returns The withdrawal fee.
 */
export async function getWithdrawalFee(
  client: Client,
  parameters: GetWithdrawalFeeParameters,
): GetWithdrawalFeeReturnType {
  const { height = 0, denom, remote } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg = { withdrawal_fee: { denom, remote } };

  return await queryWasmSmart(client, { contract: addresses.gateway, msg, height });
}
