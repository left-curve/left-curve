import type {
  Chain,
  Client,
  Signer,
  SimulateRequest,
  SimulateResponse,
  Transport,
} from "../types/index.js";

export type SimulateParameters = {
  simulate: SimulateRequest;
  scale?: number;
  base?: number;
  height?: number;
};

export type SimulateReturnType = Promise<SimulateResponse>;

/**
 * Simulate a transaction.
 * @param parameters
 * @param parameters.simulate The simulation request.
 * @param parameters.scale The scale factor to apply to the gas used.
 * @param parameters.base Base increase to apply for signature verification.
 * @param parameters.height The height at which to simulate the transaction.
 * @returns The simulation response.
 */
export async function simulate<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: SimulateParameters,
): SimulateReturnType {
  const { simulate: tx, scale = 1.3, base = 1_880_000, height = 0 } = parameters;

  const { gasLimit, gasUsed } = await client.request({
    method: "simulate",
    params: {
      tx,
      height,
      prove: false,
    },
  });

  return {
    gasLimit,
    gasUsed: Math.round((gasUsed + base) * scale),
  };
}
