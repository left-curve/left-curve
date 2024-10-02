import { decodeBase64, deserialize, serialize } from "@leftcurve/encoding";

import type {
  Chain,
  Client,
  Signer,
  SimulateRequest,
  SimulateResponse,
  Transport,
} from "@leftcurve/types";
import { queryAbci } from "./queryAbci";

export type SimulateParameters = {
  simulate: SimulateRequest;
  height?: number;
};

export type SimulateReturnType = Promise<SimulateResponse>;

/**
 * Simulate a transaction.
 * @param parameters
 * @param parameters.simulate The simulation request.
 * @param parameters.height The height at which to simulate the transaction.
 * @returns The simulation response.
 */
export async function simulate<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateParameters,
): SimulateReturnType {
  const { simulate, height = 0 } = parameters;

  const { value } = await queryAbci(client, {
    data: serialize(simulate),
    height,
    path: "/simulate",
    prove: false,
  });

  return deserialize<SimulateResponse>(decodeBase64(value ?? ""));
}
