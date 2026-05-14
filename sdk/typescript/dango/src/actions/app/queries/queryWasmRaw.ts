import type { Address, Base64, Client, WasmRawResponse } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type QueryWasmRawParameters = {
  contract: Address;
  key: Base64;
  height?: number;
};

export type QueryWasmRawReturnType = Promise<WasmRawResponse>;

/**
 * Query the raw wasm data from a contract.
 * @param parameters
 * @param parameters.contract The contract address.
 * @param parameters.key The key to query.
 * @param parameters.height The height at which to query the data.
 * @returns The raw wasm data.
 */
export async function queryWasmRaw(
  client: Client,
  parameters: QueryWasmRawParameters,
): QueryWasmRawReturnType {
  const { contract, key, height = 0 } = parameters;
  const query = {
    wasmRaw: { contract, key },
  };

  const res = await queryApp(client, { query, height });

  if (!("wasmRaw" in res)) {
    throw new Error(`expecting wasm raw response, got ${JSON.stringify(res)}`);
  }

  return res.wasmRaw;
}
