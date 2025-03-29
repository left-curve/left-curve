import type {
  Address,
  Chain,
  Client,
  Json,
  JsonValue,
  Signer,
  Transport,
  WasmSmartResponse,
} from "../types/index.js";
import { getAction } from "./getAction.js";
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
export async function queryWasmSmart<
  value extends JsonValue = JsonValue,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: QueryWasmSmartParameters,
): QueryWasmSmartReturnType<value> {
  const { contract, msg, height = 0 } = parameters;
  const query = {
    wasmSmart: { contract, msg },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if (!("wasmSmart" in res)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(res)}`);
  }

  return res.wasmSmart as value;
}
