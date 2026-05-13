import type { Address, Chain, Client, ContractInfo, Signer, Transport } from "../types/index.js";
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
export async function getContractInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
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
