import { deserialize, serialize } from "@leftcurve/encoding";

import type {
  Account,
  Chain,
  Client,
  QueryRequest,
  QueryResponse,
  Transport,
} from "@leftcurve/types";

export type QueryAppParameters = {
  query: QueryRequest;
  height?: number;
};

export type QueryAppReturnType = Promise<QueryResponse>;

export async function queryApp<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(client: Client<Transport, chain, account>, parameters: QueryAppParameters): QueryAppReturnType {
  const { query, height = 0 } = parameters;
  const res = await client.query("/app", serialize(query), height, false);
  return deserialize<QueryResponse>(res.value);
}
