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

export type GetAccountsByUsernameParameters = {
  username: Username;
  startAfter?: Address;
  limit?: number;
  height?: number;
};

export type GetAccountsByUsernameReturnType = Promise<Record<Address, AccountTypes>>;

/**
 * Enumerate all accounts associated with a username.
 * @param parameters
 * @param parameters.username The username to get accounts for.
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The maximum number of accounts to return.
 * @param parameters.height The height at which to get the accounts.
 * @returns The accounts.
 */
export async function getAccountsByUsername<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountsByUsernameParameters,
): GetAccountsByUsernameReturnType {
  const { username, startAfter, limit, height = 0 } = parameters;
  const msg = { accounts: { username, startAfter, limit } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
