import type { AccountId, Address, Chain, Client, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountIdByAddressParameters = {
  address: Address;
  height?: number;
};

export type GetAccountIdByAddressReturnType = Promise<AccountId>;

/**
 * Get the account id by address.
 * @param parameters
 * @param parameters.address The address of the account to get information for.
 * @param parameters.height The height at which to get the account id.
 * @returns The account id.
 */
export async function getAccountIdByAddress<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountIdByAddressParameters,
): GetAccountIdByAddressReturnType {
  const { address, height = 0 } = parameters;
  const msg = { accountId: { address } };

  const accountFactory = await getAppConfig<Address>(client, {
    key: "account_factory",
  });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
