import { queryApp as internalQueryApp } from "@left-curve/sdk/actions";
import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/sdk/encoding";
import { gql } from "graphql-request";
import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { QueryAppParameters, QueryAppReturnType } from "@left-curve/sdk";
import type { Client, QueryResponse, Transport } from "@left-curve/sdk/types";
import type { Chain } from "../../../types/chain.js";
import type { Signer } from "../../../types/signer.js";

/**
 * Query the application state.
 * @param parameters
 * @param parameters.query The query request.
 * @param parameters.height The height at which to query the application state.
 * @returns The query response.
 */
export async function queryApp<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(client: Client<Transport, chain, signer>, parameters: QueryAppParameters): QueryAppReturnType {
  const { query, height = 0 } = parameters;
  const { transport } = client;

  if (transport.type !== "http-graphql") return await internalQueryApp(client, parameters);

  const document = gql`
    query queryResult($request: String!, $height: Int!) {
      queryApp(request: $request, height: $height)
    }
  `;

  const { queryApp: response } = await queryIndexer<{ queryApp: QueryResponse }, chain, signer>(
    client,
    {
      document,
      variables: {
        request: snakeCaseJsonSerialization(query),
        height,
      },
    },
  );

  return camelCaseJsonDeserialization<QueryResponse>(response);
}
