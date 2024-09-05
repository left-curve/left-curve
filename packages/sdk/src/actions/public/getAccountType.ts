import type {
  AccountType,
  Address,
  Chain,
  Client,
  Signer,
  Transport,
  Username,
} from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountTypeParameters = {
  username: Username;
  address: Address;
  height?: number;
};

export type GetAccountTypeReturnType = Promise<AccountType>;

/**
 * Get the account type by username and address
 * @param parameters
 * @param parameters.username The username of the account.
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to get the account type.
 * @returns The account type.
 */
export async function getAccountType<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountTypeParameters,
): GetAccountTypeReturnType {
  const { username, address, height = 0 } = parameters;
  const msg = { account: { username, address } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
