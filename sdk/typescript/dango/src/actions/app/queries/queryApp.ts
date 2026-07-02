import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";
import type { Client, Json, QueryResponse } from "@left-curve/types";
import { queryIndexer } from "#actions/indexer/queryIndexer.js";

export type QueryAppParameters = {
  query: Json;
};

export type QueryAppReturnType = Promise<QueryResponse>;

export async function queryApp(client: Client, parameters: QueryAppParameters): QueryAppReturnType {
  const { query } = parameters;

  const document = `
    query queryResult($request: String!) {
      queryApp(request: $request)
    }
  `;

  const { queryApp: response } = await queryIndexer<{ queryApp: QueryResponse }>(client, {
    document,
    variables: {
      request: snakeCaseJsonSerialization(query),
    },
  });

  return camelCaseJsonDeserialization<QueryResponse>(response);
}
