import type { AccountType, Address, Chain, Client, Hex, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountTypeCodeHashParameters = {
  accountType: AccountType;
  height?: number;
};

export type GetAccountTypeCodeHashReturnType = Promise<Hex>;

/**
 * Get the account type code hash.
 * @param parameters
 * @param parameters.accountType The account type.
 * @param parameters.height The height at which to query the account type code hash.
 * @returns The account type code hash.
 */
export async function getAccountTypeCodeHash<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountTypeCodeHashParameters,
): GetAccountTypeCodeHashReturnType {
  const { accountType, height = 0 } = parameters;
  const msg = {
    codeHash: { accountType },
  };
  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
