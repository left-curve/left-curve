import { encodeBase64 } from "@leftcurve/encoding";
import type { Account, Address, Chain, Client, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type QueryWasmRawParameters = {
  contract: Address;
  key: Uint8Array;
  height?: number;
};

export type QueryWasmRawReturnType<value extends any | undefined> = Promise<value>;

/**
 * Query the raw wasm data from a contract.
 * @param parameters
 * @param parameters.contract The contract address.
 * @param parameters.key The key to query.
 * @param parameters.height The height at which to query the data.
 * @returns The raw wasm data.
 */
export async function queryWasmRaw<
  value extends any | undefined = any | undefined,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: QueryWasmRawParameters,
): QueryWasmRawReturnType<value> {
  const { contract, key, height = 0 } = parameters;
  const query = {
    wasmRaw: { contract, key: encodeBase64(key) },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if (!("wasmRaw" in res)) {
    throw new Error(`expecting wasm raw response, got ${JSON.stringify(res)}`);
  }

  return res.wasmRaw as unknown as value;
}
