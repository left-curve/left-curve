import type { Address, Base64, Client, WasmRawResponse } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type QueryWasmRawParameters = {
  contract: Address;
  key: Base64;
};

export type QueryWasmRawReturnType = Promise<WasmRawResponse>;

/**
 * Query the raw wasm data from a contract.
 * @param parameters
 * @param parameters.contract The contract address.
 * @param parameters.key The key to query.
 * @returns The raw wasm data.
 */
export async function queryWasmRaw(
  client: Client,
  parameters: QueryWasmRawParameters,
): QueryWasmRawReturnType {
  const { contract, key } = parameters;
  const query = {
    wasmRaw: { contract, key },
  };

  const res = await queryApp(client, { query });

  if (!("wasmRaw" in res)) {
    throw new Error(`expecting wasm raw response, got ${JSON.stringify(res)}`);
  }

  return res.wasmRaw;
}
