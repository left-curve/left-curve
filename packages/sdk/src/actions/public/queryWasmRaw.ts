import { encodeBase64 } from "@leftcurve/encoding";
import type { Account, Chain, Client, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type QueryWasmRawParameters = {
  contract: string;
  key: Uint8Array;
  height?: number;
};

// biome-ignore lint/suspicious/noExplicitAny: It could be any time
export type QueryWasmRawReturnType<value extends any | undefined> = Promise<value>;

export async function queryWasmRaw<
  // biome-ignore lint/suspicious/noExplicitAny: It could be any time
  value extends any | undefined,
  chain extends Chain | undefined,
  account extends Account | undefined,
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
