import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import type { Address, Chain, Client, Signer, Transport } from "@left-curve/types";
import type { AccountTypes, AppConfig, Username } from "../../../types/index.js";

export type GetNextAccountAddressParameters = {
  username: Username;
  accountType: AccountTypes;
  height?: number;
};

export type GetNextAccountAddressReturnType = Promise<Address>;

/**
 * @param parameters
 * @param parameters.username The username
 * @param parameters.accountType The account type
 * @param parameters.height The height at which to get the accounts.
 * @returns The new address.
 */
export async function getNextAccountAddress<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetNextAccountAddressParameters,
): GetNextAccountAddressReturnType {
  const { username, accountType, height = 0 } = parameters;
  const msg = { nextAccountAddress: { username, accountType } };

  const { addresses } = await getAppConfig<AppConfig>(client);

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
