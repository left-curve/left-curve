import type { Account, Chain, Client, Json, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type QueryWasmSmartParameters = {
  contract: string;
  msg: Json;
  height?: number;
};

// biome-ignore lint/suspicious/noExplicitAny: It could be any time
export type QueryWasmSmartReturnType<value extends any | undefined> = Promise<value>;

export async function queryWasmSmart<
  // biome-ignore lint/suspicious/noExplicitAny: It could be any time
  value extends any | undefined,
  chain extends Chain | undefined,
  account extends Account | undefined,
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
