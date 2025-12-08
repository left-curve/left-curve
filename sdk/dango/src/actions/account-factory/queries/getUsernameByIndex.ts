import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";

import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig, Username } from "../../../types/index.js";

export type GetUsernameByIndexParameters = {
  index: number;
  height?: number;
};

export type GetUsernameByIndexReturnType = Promise<Username>;

/**
 * Given an index get the username.
 * @param parameters
 * @param parameters.index The index of the user.
 * @param parameters.height The height at which to get the username.
 * @returns The username
 */
export async function getUsernameByIndex<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetUsernameByIndexParameters,
): GetUsernameByIndexReturnType {
  const { index, height = 0 } = parameters;
  const msg = { userNameByIndex: index };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
