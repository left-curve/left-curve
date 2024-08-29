import type {
  Account,
  Address,
  Chain,
  Client,
  ContractsResponse,
  Transport,
} from "@leftcurve/types";
import { queryApp } from "./queryApp";

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
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters?: GetContractsInfoParameters,
): GetContractsInfoReturnType {
  const { startAfter, limit, height = 0 } = parameters || {};
  const query = {
    contracts: { startAfter, limit },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if (!("contracts" in res)) {
    throw new Error(`expecting contracts response, got ${JSON.stringify(res)}`);
  }

  return res.contracts;
}
