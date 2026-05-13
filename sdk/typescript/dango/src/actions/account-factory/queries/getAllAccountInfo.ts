import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import type { Address, Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { AccountInfo, AppConfig } from "../../../types/index.js";

export type GetAllAccountInfoParameters = {
  startAfter?: Address;
  limit?: number;
  height?: number;
};

export type GetAllAccountInfoReturnType = Promise<Record<Address, AccountInfo>>;

/**
 * Get all account info in the factory.
 * @param parameters
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The maximum number of accounts to return.
 * @param parameters.height The height at which to get all account info.
 * @returns A record of address and account info.
 */
export async function getAllAccountInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAllAccountInfoParameters,
): GetAllAccountInfoReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg = { accounts: { startAfter, limit } };

  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
