import { type QueryBlockParameters, type QueryBlockReturnType, queryBlock } from "./queryBlock.js";

import type { Client, Transport } from "@left-curve/sdk/types";

export type IndexerActions = {
  queryBlock: (args: QueryBlockParameters) => QueryBlockReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: Client<transport>,
): IndexerActions {
  return {
    queryBlock: (args) => queryBlock(client, args),
  };
}
