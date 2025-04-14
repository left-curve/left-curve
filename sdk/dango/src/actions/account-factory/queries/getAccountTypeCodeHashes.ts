import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Hex, Signer, Transport } from "@left-curve/sdk/types";
import type { AccountTypes, AppConfig } from "../../../types/index.js";

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
  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
