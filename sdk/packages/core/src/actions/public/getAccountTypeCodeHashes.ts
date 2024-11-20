import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

import type { AccountTypes, Chain, Client, Hex, Signer, Transport } from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

export type GetAccountTypeCodeHashesParameters = {
  limit?: number;
  startAfter?: AccountTypes;
  height?: number;
};

export type GetAccountTypeCodeHashesReturnType = Promise<Record<AccountTypes, Hex>>;

/**
 * Get the account type code hashes.
 * @param parameters
 * @param parameters.limit The number of account types to return.
 * @param parameters.startAfter The account type to start after.
 * @param parameters.height The height at which to query the account type code hashes.
 * @returns The account type code hashes.
 */
export async function getAccountTypeCodeHashes<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters?: GetAccountTypeCodeHashesParameters,
): GetAccountTypeCodeHashesReturnType {
  const { startAfter, limit, height = 0 } = parameters || {};
  const msg = {
    codeHashes: { startAfter, limit },
  };
  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
