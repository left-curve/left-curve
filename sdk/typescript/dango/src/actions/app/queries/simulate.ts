import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";
import type { Client, SimulateRequest, SimulateResponse } from "@left-curve/types";
import { queryIndexer } from "#actions/indexer/queryIndexer.js";

export type SimulateParameters = {
  simulate: SimulateRequest;
  scale?: number;
  height?: number;
};

export type SimulateReturnType = Promise<SimulateResponse>;

export async function simulate(client: Client, parameters: SimulateParameters): SimulateReturnType {
  const { simulate, scale = 1.3, height = 0 } = parameters;

  const document = `
    query simulateResult($tx: String!)  {
      simulate(tx: $tx)
    }
  `;

  const { simulate: response } = await queryIndexer<{ simulate: SimulateResponse }>(client, {
    document,
    variables: {
      tx: snakeCaseJsonSerialization(simulate),
    },
  });

  const simulation = camelCaseJsonDeserialization<SimulateResponse>(response);

  const { gasLimit, gasUsed } = simulation;

  return {
    gasLimit,
    gasUsed: Math.round(gasUsed * scale),
  };
}
