import type { Client, JsonValue, Transport } from "@left-curve/sdk/types";

export type QueryIndexerParameters = {
  document: string;
  variables: Record<string, unknown>;
};

export async function queryIndexer<
  value extends JsonValue = JsonValue,
  transport extends Transport = Transport,
>(client: Client<transport>, parameters: QueryIndexerParameters): Promise<value> {
  return await client.request({
    method: "query",
    params: parameters,
  });
}
