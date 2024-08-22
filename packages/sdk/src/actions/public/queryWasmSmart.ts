import type { Account, Address, Chain, Client, Json, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type QueryWasmSmartParameters = {
  contract: Address;
  msg: Json;
  height?: number;
};

export type QueryWasmSmartReturnType<value extends any | undefined> = Promise<value>;

/**
 * Query a wasm smart contract.
 * @param parameters
 * @param parameters.contract The address of the smart contract.
 * @param parameters.msg The message to send to the smart contract.
 * @param parameters.height The height at which to query the smart contract.
 * @returns The response from the smart contract.
 */
export async function queryWasmSmart<
  value extends any | undefined = any | undefined,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: QueryWasmSmartParameters,
): QueryWasmSmartReturnType<value> {
  const { contract, msg, height = 0 } = parameters;
  const query = {
    wasmSmart: { contract, msg },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if (!("wasmSmart" in res)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(res)}`);
  }

  return res.wasmSmart as unknown as value;
}
