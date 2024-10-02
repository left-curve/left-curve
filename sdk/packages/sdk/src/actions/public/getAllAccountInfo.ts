import type { AccountInfo, Address, Chain, Client, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

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

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
