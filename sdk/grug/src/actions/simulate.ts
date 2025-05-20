import { decodeBase64, deserialize, serialize } from "../encoding/index.js";

import type {
  Chain,
  Client,
  Signer,
  SimulateRequest,
  SimulateResponse,
  Transport,
} from "../types/index.js";
import { queryAbci } from "./queryAbci.js";

export type SimulateParameters = {
  simulate: SimulateRequest;
  scale?: number;
  height?: number;
};

export type SimulateReturnType = Promise<SimulateResponse>;

/**
 * Simulate a transaction.
 * @param parameters
 * @param parameters.simulate The simulation request.
 * @param parameters.scale The scale factor to apply to the gas used.
 * @param parameters.height The height at which to simulate the transaction.
 * @returns The simulation response.
 */
export async function simulate<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateParameters,
): SimulateReturnType {
  const { simulate, scale = 1.3, height = 0 } = parameters;

  const { value } = await queryAbci(client, {
    data: serialize(simulate),
    height,
    path: "/simulate",
    prove: false,
  });

  const { gasLimit, gasUsed } = deserialize<SimulateResponse>(decodeBase64(value ?? ""));
  return {
    gasLimit,
    gasUsed: Math.round(gasUsed * scale),
  };
}
