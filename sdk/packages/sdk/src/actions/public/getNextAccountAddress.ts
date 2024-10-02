import type {
  AccountTypes,
  Address,
  Chain,
  Client,
  Signer,
  Transport,
  Username,
} from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

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

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
