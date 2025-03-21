import type { JsonValue, Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../types/clients.js";

export type QueryIndexerParameters = {
  document: string;
  variables: Record<string, unknown>;
};

export async function queryIndexer<
  value extends JsonValue = JsonValue,
  transport extends Transport = Transport,
>(client: DangoClient<transport>, parameters: QueryIndexerParameters): Promise<value> {
  return await client.request({
    method: "query",
    params: parameters,
  });
}
