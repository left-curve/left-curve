import type {
  Account,
  AccountIndex,
  AccountInfo,
  Address,
  Chain,
  Client,
  Transport,
  Username,
} from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountsByUsernameParameters = {
  username: Username;
  startAfter?: number;
  limit?: number;
  height?: number;
};

export type GetAccountsByUsernameReturnType = Promise<Record<AccountIndex, AccountInfo>>;

/**
 * Enumerate all accounts associated with a username.
 * @param parameters
 * @param parameters.username The username to get accounts for.
 * @param parameters.startAfter The account index to start after.
 * @param parameters.limit The maximum number of accounts to return.
 * @param parameters.height The height at which to get the accounts.
 * @returns The accounts.
 */
export async function getAccountsByUsername<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetAccountsByUsernameParameters,
): GetAccountsByUsernameReturnType {
  const { username, startAfter, limit, height = 0 } = parameters;
  const msg = { accounts: { username, startAfter, limit } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
