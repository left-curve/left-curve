import { queryWasmSmart } from "../../../index.js";

import type { Address, Client } from "../../../types/index.js";
import type { UserStatus } from "../../../types/account.js";

export type GetAccountStatusParameters = {
  address: Address;
  height?: number;
};

export type GetAccountStatusReturnType = Promise<UserStatus>;

export async function getAccountStatus(
  client: Client,
  parameters: GetAccountStatusParameters,
): GetAccountStatusReturnType {
  const { address, height = 0 } = parameters;
  const msg = {
    status: {},
  };

  return await queryWasmSmart(client, { contract: address, msg, height });
}
