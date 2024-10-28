import type {
  AccountIndex,
  Address,
  Chain,
  Client,
  Signer,
  Transport,
  Username,
} from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

export type GetNextAccountIndexParameters = {
  username: Username;
  height?: number;
};

export type GetNextAccountIndexReturnType = Promise<AccountIndex>;

/**
 * Query the account index, which is used in deriving the account address,
 * must be used if a user is to create a new account.
 * @param parameters
 * @param parameters.username The username referece to get the next index.
 * @param parameters.height The height at which to get the accounts.
 * @returns The index.
 */
export async function getNextAccountIndex<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetNextAccountIndexParameters,
): GetNextAccountIndexReturnType {
  const { username, height = 0 } = parameters;
  const msg = { nextAccountIndex: { username } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
