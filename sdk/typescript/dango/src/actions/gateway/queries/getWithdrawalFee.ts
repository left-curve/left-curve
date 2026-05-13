import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Denom, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { Remote } from "../../../types/hyperlane.js";

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
export async function getWithdrawalFee<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetWithdrawalFeeParameters,
): GetWithdrawalFeeReturnType {
  const { height = 0, denom, remote } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  const msg = { withdrawal_fee: { denom, remote } };

  return await queryWasmSmart(client, { contract: addresses.gateway, msg, height });
}
