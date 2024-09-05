import type { Address, Chain, Client, Hex, Signer, Transport, Username } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetUsersByKeyhashParameters = {
  hash: Hex;
  height?: number;
};

export type GetUsersByKeyHashReturnType = Promise<Username[]>;

/**
 * Given a key hash, get the key id.
 * @param parameters
 * @param parameters.hash The key hash of the account.
 * @returns an array of usernames.
 */
export async function getUsersByKeyHash<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetUsersByKeyhashParameters,
): GetUsersByKeyHashReturnType {
  const { hash, height = 0 } = parameters;
  const msg = { usersByKey: { hash } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
