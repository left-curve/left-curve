import type { Address, Client, Json, JsonValue, WasmSmartResponse } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type QueryWasmSmartParameters = {
  contract: Address;
  msg: Json;
  height?: number;
};

export type QueryWasmSmartReturnType<T = JsonValue> = Promise<WasmSmartResponse<T>>;

/**
 * Query a wasm smart contract.
 * @param parameters
 * @param parameters.contract The address of the smart contract.
 * @param parameters.msg The message to send to the smart contract.
 * @param parameters.height The height at which to query the smart contract.
 * @returns The response from the smart contract.
 */
export async function queryWasmSmart<value extends JsonValue = JsonValue>(
  client: Client,
  parameters: QueryWasmSmartParameters,
): QueryWasmSmartReturnType<value> {
  const { contract, msg, height = 0 } = parameters;
  const query = {
    wasmSmart: { contract, msg },
  };

  const res = await queryApp(client, { query, height });

  if (!("wasmSmart" in res)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(res)}`);
  }

  return res.wasmSmart as value;
}
