import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Hex, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, Username } from "../../../types/index.js";

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

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
