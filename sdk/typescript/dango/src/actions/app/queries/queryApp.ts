import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "../../../encoding/index.js";
import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client, Json, QueryResponse } from "../../../types/index.js";

export type QueryAppParameters = {
  query: Json;
  height?: number;
};

export type QueryAppReturnType = Promise<QueryResponse>;

export async function queryApp(client: Client, parameters: QueryAppParameters): QueryAppReturnType {
  const { query, height } = parameters;

  const document = `
    query queryResult($request: String!, $height: Int) {
      queryApp(request: $request, height: $height)
    }
  `;

  const { queryApp: response } = await queryIndexer<{ queryApp: QueryResponse }>(client, {
    document,
    variables: {
      request: snakeCaseJsonSerialization(query),
      height: height === 0 ? undefined : height,
    },
  });

  return camelCaseJsonDeserialization<QueryResponse>(response);
}
