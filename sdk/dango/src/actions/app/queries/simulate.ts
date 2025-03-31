import { queryAbci } from "@left-curve/sdk/actions";
import {
  camelCaseJsonDeserialization,
  decodeBase64,
  deserialize,
  snakeCaseJsonSerialization,
} from "@left-curve/sdk/encoding";
import { serialize } from "@left-curve/sdk/encoding";

import type { SimulateParameters, SimulateReturnType } from "@left-curve/sdk";
import type { Client, QueryAbciResponse, SimulateResponse, Transport } from "@left-curve/sdk/types";
import { gql } from "graphql-request";
import type { Chain } from "../../../types/chain.js";
import type { Signer } from "../../../types/signer.js";
import { queryIndexer } from "../../indexer/queryIndexer.js";

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
  const { simulate, scale = 1.3, base = 1_880_000, height = 0 } = parameters;
  const { transport } = client;

  const simulation = await (async () => {
    if (transport.type !== "http-graphql") {
      const { value } = await queryAbci(client, {
        data: serialize(simulate),
        height,
        path: "/simulate",
        prove: false,
      });

      return deserialize<SimulateResponse>(decodeBase64(value ?? ""));
    }

    const document = gql`
      query simulateResult($tx: String!)  {
        simulate(tx: $tx)
      }
    `;

    const { simulate: response } = await queryIndexer<{ simulate: string }, chain, signer>(client, {
      document,
      variables: {
        tx: snakeCaseJsonSerialization(simulate),
      },
    });

    return camelCaseJsonDeserialization<SimulateResponse>(response);
  })();

  const { gasLimit, gasUsed } = simulation;

  return {
    gasLimit,
    gasUsed: Math.round((gasUsed + base) * scale),
  };
}
