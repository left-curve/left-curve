import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import type { AccountIndex, AppConfig, Username } from "../../../types/index.js";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Signer, Transport } from "@left-curve/sdk/types";

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

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
