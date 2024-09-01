import type { AccountType, Address, Chain, Client, Hex, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountTypeCodeHashesParameters = {
  limit?: number;
  startAfter?: AccountType;
  height?: number;
};

export type GetAccountTypeCodeHashesReturnType = Promise<Record<AccountType, Hex>>;

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
  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
