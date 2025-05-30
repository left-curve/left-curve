import { type QueryBlockParameters, type QueryBlockReturnType, queryBlock } from "./queryBlock.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import { type SearchTxParameters, type SearchTxReturnType, searchTx } from "./searchTx.js";

import {
  type BlockSubscriptionParameters,
  type BlockSubscriptionReturnType,
  blockSubscription,
} from "./subscriptions/block.js";

import {
  type TransferSubscriptionParameters,
  type TransferSubscriptionReturnType,
  transferSubscription,
} from "./subscriptions/transfer.js";

export type IndexerActions = {
  queryBlock: (args?: QueryBlockParameters) => QueryBlockReturnType;
  searchTx: (args: SearchTxParameters) => SearchTxReturnType;
  blockSubscription: (args: BlockSubscriptionParameters) => BlockSubscriptionReturnType;
  transferSubscription: (args: TransferSubscriptionParameters) => TransferSubscriptionReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: Client<transport>,
): IndexerActions {
  return {
    searchTx: (args) => searchTx(client, args),
    queryBlock: (args) => queryBlock(client, args),
    blockSubscription: (args) => blockSubscription(client, args),
    transferSubscription: (args) => transferSubscription(client, args),
  };
}
