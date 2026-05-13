import type { Address, Client, ContractInfo } from "../types/index.js";
import { getAction } from "./getAction.js";
import { queryApp } from "./queryApp.js";

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
export async function getContractInfo(
  client: Client,
  parameters: GetContractInfoParameters,
): GetContractInfoReturnType {
  const { address, height = 0 } = parameters;
  const query = {
    contract: { address },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if (!("contract" in res)) {
    throw new Error(`expecting contract response, got ${JSON.stringify(res)}`);
  }

  return res.contract;
}
