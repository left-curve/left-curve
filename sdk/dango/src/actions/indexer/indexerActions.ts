import { type QueryBlockParameters, type QueryBlockReturnType, queryBlock } from "./queryBlock.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import { type SearchTxParameters, type SearchTxReturnType, searchTx } from "./searchTx.js";

export type IndexerActions = {
  queryBlock: (args: QueryBlockParameters) => QueryBlockReturnType;
  searchTx: (args: SearchTxParameters) => SearchTxReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: Client<transport>,
): IndexerActions {
  return {
    searchTx: (args) => searchTx(client, args),
    queryBlock: (args) => queryBlock(client, args),
  };
}
