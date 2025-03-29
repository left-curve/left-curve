import type { Client, JsonValue, Transport } from "@left-curve/sdk/types";
import type { Chain } from "../../types/chain.js";
import type { GraphQLSchemaOverride } from "../../types/graphql.js";
import type { Signer } from "../../types/signer.js";

export type QueryIndexerParameters = {
  document: string;
  variables?: Record<string, unknown>;
};

export async function queryIndexer<
  value extends JsonValue = JsonValue,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(client: Client<Transport, chain, signer>, parameters: QueryIndexerParameters): Promise<value> {
  const { document, variables } = parameters;

  return client.request<GraphQLSchemaOverride<value>>({
    method: document,
    params: variables,
  });
}
