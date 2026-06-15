import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";

import type { Address, Client, UserStatus } from "@left-curve/types";

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
