import type { Address, Client, Json, JsonValue, WasmSmartResponse } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type QueryWasmSmartParameters = {
  contract: Address;
  msg: Json;
};

export type QueryWasmSmartReturnType<T = JsonValue> = Promise<WasmSmartResponse<T>>;

/**
 * Query a wasm smart contract.
 * @param parameters
 * @param parameters.contract The address of the smart contract.
 * @param parameters.msg The message to send to the smart contract.
 * @returns The response from the smart contract.
 */
export async function queryWasmSmart<value extends JsonValue = JsonValue>(
  client: Client,
  parameters: QueryWasmSmartParameters,
): QueryWasmSmartReturnType<value> {
  const { contract, msg } = parameters;
  const query = {
    wasmSmart: { contract, msg },
  };

  const res = await queryApp(client, { query });

  if (!("wasmSmart" in res)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(res)}`);
  }

  return res.wasmSmart as value;
}
