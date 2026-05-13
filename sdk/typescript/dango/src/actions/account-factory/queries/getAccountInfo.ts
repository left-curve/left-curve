import { getAction, getAppConfig, queryWasmSmart } from "@left-curve/sdk/actions";
import { getUser } from "./getUser.js";

import type { Address, Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AccountDetails, AccountInfo, AppConfig } from "../../../types/index.js";

export type GetAccountInfoParameters = {
  address: Address;
  height?: number;
};

export type GetAccountInfoReturnType = Promise<AccountDetails | null>;

/**
 * Given an account address get the account info.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.height The height at which to get the account info.
 * @returns The account info.
 */
export async function getAccountInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountInfoParameters,
): GetAccountInfoReturnType {
  const { address, height = 0 } = parameters;
  const msg = { account: { address } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  const account = await queryWasmSmart<AccountInfo, chain, signer>(client, {
    contract: addresses.accountFactory,
    msg,
    height,
  });

  if (!account) return null;

  const user = await getUser(client, { userIndexOrName: { index: account.owner }, height });

  return {
    ...account,
    username: user.name,
    address: parameters.address,
  };
}
