import type {
  AccountInfo,
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
  height?: number;
};

export type GetAccountsByUsernameReturnType = Promise<Record<Address, AccountInfo>>;

/**
 * Find all accounts associated with a user.
 * @param parameters
 * @param parameters.username The username to get accounts for.
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
  const { username, height = 0 } = parameters;
  const msg = { accountsByUser: { username } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
