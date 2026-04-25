import type {
  Address,
  Chain,
  Client,
  ContractsResponse,
  Signer,
  Transport,
} from "../types/index.js";
import { getAction } from "./getAction.js";
import { queryApp } from "./queryApp.js";

export type GetContractsInfoParameters = {
  startAfter?: Address;
  limit?: number;
  height?: number;
};

export type GetContractsInfoReturnType = Promise<ContractsResponse>;

/**
 * Get the contracts.
 * @param parameters
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The number of contracts to return.
 * @param parameters.height The height at which to query the contracts.
 * @returns The contracts.
 */
export async function getContractsInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters?: GetContractsInfoParameters,
): GetContractsInfoReturnType {
  const { startAfter, limit, height = 0 } = parameters || {};
  const query = {
    contracts: { startAfter, limit },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if (!("contracts" in res)) {
    throw new Error(`expecting contracts response, got ${JSON.stringify(res)}`);
  }

  return res.contracts;
}
