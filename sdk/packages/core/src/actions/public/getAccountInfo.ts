import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

import type { AccountInfo, Address, Chain, Client, Signer, Transport } from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type GetAccountInfoParameters = {
  address: Address;
  height?: number;
};

export type GetAccountInfoReturnType = Promise<AccountInfo>;

/**
 * Given an account address get the account info.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to get the account info.
 * @returns The account info.
 */
export async function getAccountInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountInfoParameters,
): GetAccountInfoReturnType {
  const { address, height = 0 } = parameters;
  const msg = { account: { address } };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
