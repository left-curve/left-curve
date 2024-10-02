import { decodeBase64, deserialize, serialize } from "@leftcurve/encoding";

import type {
  Chain,
  Client,
  QueryRequest,
  QueryResponse,
  Signer,
  Transport,
} from "@leftcurve/types";
import { queryAbci } from "./queryAbci";

export type QueryAppParameters = {
  query: QueryRequest;
  height?: number;
};

export type QueryAppReturnType = Promise<QueryResponse>;

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

  const { value } = await queryAbci(client, {
    data: serialize(query),
    height,
    path: "/app",
    prove: false,
  });

  return deserialize<QueryResponse>(decodeBase64(value ?? ""));
}
