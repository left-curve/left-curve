import type { Account, Address, Chain, Client, Hex, Transport, Username } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountsByKeyHashParameters = {
  hash: Hex;
  height?: number;
};

export type GetAccountsByKeyHashReturnType = Promise<Username[]>;

/**
 * Given a key id, look up the usernames associated with this account.
 * @param parameters
 * @param parameters.hash The key hash of the account.
 * @returns The usernames associated with the key id.
 */
export async function getAccountsByKeyHash<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetAccountsByKeyHashParameters,
): GetAccountsByKeyHashReturnType {
  const { hash, height = 0 } = parameters;
  const msg = { keyId: { hash } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
