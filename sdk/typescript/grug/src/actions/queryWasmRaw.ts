import type {
  Address,
  Base64,
  Chain,
  Client,
  Signer,
  Transport,
  WasmRawResponse,
} from "../types/index.js";
import { getAction } from "./getAction.js";
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
export async function queryWasmRaw<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: QueryWasmRawParameters,
): QueryWasmRawReturnType {
  const { contract, key, height = 0 } = parameters;
  const query = {
    wasmRaw: { contract, key },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if (!("wasmRaw" in res)) {
    throw new Error(`expecting wasm raw response, got ${JSON.stringify(res)}`);
  }

  return res.wasmRaw;
}
