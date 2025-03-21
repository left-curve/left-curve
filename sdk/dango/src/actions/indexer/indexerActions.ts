import type { DangoClient, PublicClient } from "../../types/clients.js";
import { type QueryBlockParameters, type QueryBlockReturnType, queryBlock } from "./queryBlock.js";

import type { Transport } from "@left-curve/sdk/types";

export type IndexerActions = {
  queryBlock: (args: QueryBlockParameters) => QueryBlockReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: DangoClient<transport>,
): IndexerActions {
  return {
    queryBlock: (args) => queryBlock(client, args),
  };
}
