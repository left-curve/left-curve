import type { Client, JsonValue } from "../../types/index.js";

export type QueryIndexerParameters = {
  document: string;
  variables?: Record<string, unknown>;
};

export async function queryIndexer<value extends JsonValue = JsonValue>(
  client: Client,
  parameters: QueryIndexerParameters,
): Promise<value> {
  const { document, variables } = parameters;

  return client.request<value>({
    request: document,
    params: variables,
  });
}
