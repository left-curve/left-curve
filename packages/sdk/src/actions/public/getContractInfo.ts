import type { Account, Address, Chain, Client, ContractInfo, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetContractInfoParameters = {
  address: Address;
  height?: number;
};

export type GetContractInfoReturnType = Promise<ContractInfo>;

/**
 * Get the contract info.
 * @param parameters
 * @param parameters.address The address of the contract.
 * @param parameters.height The height at which to query the contract info.
 * @returns The contract info.
 */
export async function getContractInfo<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetContractInfoParameters,
): GetContractInfoReturnType {
  const { address, height = 0 } = parameters;
  const query = {
    contract: { address },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if (!("contract" in res)) {
    throw new Error(`expecting contract response, got ${JSON.stringify(res)}`);
  }

  return res.contract;
}
