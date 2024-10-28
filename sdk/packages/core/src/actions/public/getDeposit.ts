import type { Address, Chain, Client, Coins, Signer, Transport } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig.js";
import { queryWasmSmart } from "./queryWasmSmart.js";

export type GetDepositParameters = {
  recipient: Address;
  height?: number;
};

export type GetDepositReturnType = Promise<Coins>;

/**
 * Query unclaimed deposit for the given address.
 * @param parameters
 * @param parameters.recipient The address of the recipient.
 * @param parameters.height The height at which to get the deposit.
 * @returns The deposit.
 */
export async function getDeposit<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetDepositParameters,
): GetDepositReturnType {
  const { recipient, height = 0 } = parameters;
  const msg = { deposit: { recipient } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
