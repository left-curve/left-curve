import { queryWasmSmart } from "@left-curve/sdk";

import type { Address, Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { UserStatus } from "../../../types/account.js";

export type GetAccountStatusParameters = {
  address: Address;
  height?: number;
};

export type GetAccountStatusReturnType = Promise<UserStatus>;

export async function getAccountStatus<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAccountStatusParameters,
): GetAccountStatusReturnType {
  const { address, height = 0 } = parameters;
  const msg = {
    status: {},
  };

  return await queryWasmSmart(client, { contract: address, msg, height });
}
