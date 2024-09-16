import type { Address, Chain, Client, Coins, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetDepositsParameters = {
  startAfter?: Address;
  limit?: number;
  height?: number;
};

export type GetDepositsReturnType = Promise<Record<Address, Coins>>;

/**
 * Query all deposits in the factory
 * @param parameters
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The maximum number of deposits to return.
 * @param parameters.height The height at which to get the deposit.
 * @returns A record of recipient addresses and their deposits.
 */
export async function getDeposits<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetDepositsParameters,
): GetDepositsReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg = { deposits: { startAfter, limit } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
