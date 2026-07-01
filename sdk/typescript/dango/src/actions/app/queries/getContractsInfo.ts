import type { Address, Client, ContractsResponse } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type GetContractsInfoParameters = {
  startAfter?: Address;
  limit?: number;
};

export type GetContractsInfoReturnType = Promise<ContractsResponse>;

/**
 * Get the contracts.
 * @param parameters
 * @param parameters.startAfter The address to start after.
 * @param parameters.limit The number of contracts to return.
 * @returns The contracts.
 */
export async function getContractsInfo(
  client: Client,
  parameters?: GetContractsInfoParameters,
): GetContractsInfoReturnType {
  const { startAfter, limit } = parameters || {};
  const query = {
    contracts: { startAfter, limit },
  };

  const res = await queryApp(client, { query });

  if (!("contracts" in res)) {
    throw new Error(`expecting contracts response, got ${JSON.stringify(res)}`);
  }

  return res.contracts;
}
