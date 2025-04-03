import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Hex, Signer, Transport } from "@left-curve/sdk/types";
import type { AccountTypes, AppConfig } from "../../../types/index.js";

export type GetAccountTypeCodeHashParameters = {
  accountType: AccountTypes;
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
  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
