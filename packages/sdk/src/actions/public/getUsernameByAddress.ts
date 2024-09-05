import type { Address, Chain, Client, Signer, Transport, Username } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetUsernameByAddressParameters = {
  address: Address;
  height?: number;
};

export type GetUsernameByAddressreturnType = Promise<Username>;

/**
 * Given an account address, look up the usernames associated with it.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to get the account id.
 * @returns username
 */
export async function getUsernameByAddress<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetUsernameByAddressParameters,
): GetUsernameByAddressreturnType {
  const { address, height = 0 } = parameters;
  const msg = { usersByAddress: { address } };

  const accountFactory = await getAppConfig<Address>(client, {
    key: "account_factory",
  });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
