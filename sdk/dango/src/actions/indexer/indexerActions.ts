import { type QueryBlockParameters, type QueryBlockReturnType, queryBlock } from "./queryBlock.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import { type SearchTxsParameters, type SearchTxsReturnType, searchTxs } from "./searchTxs.js";

import {
  type BlockSubscriptionParameters,
  type BlockSubscriptionReturnType,
  blockSubscription,
} from "./subscriptions/block.js";

import {
  type AccountSubscriptionParameters,
  type AccountSubscriptionReturnType,
  accountSubscription,
} from "./subscriptions/account.js";

import {
  type TransferSubscriptionParameters,
  type TransferSubscriptionReturnType,
  transferSubscription,
} from "./subscriptions/transfer.js";

export type IndexerActions = {
  queryBlock: (args?: QueryBlockParameters) => QueryBlockReturnType;
  searchTxs: (args: SearchTxsParameters) => SearchTxsReturnType;
  blockSubscription: (args: BlockSubscriptionParameters) => BlockSubscriptionReturnType;
  transferSubscription: (args: TransferSubscriptionParameters) => TransferSubscriptionReturnType;
  accountSubscription: (args: AccountSubscriptionParameters) => AccountSubscriptionReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: Client<transport>,
): IndexerActions {
  return {
    searchTxs: (args) => searchTxs(client, args),
    queryBlock: (args) => queryBlock(client, args),
    blockSubscription: (args) => blockSubscription(client, args),
    transferSubscription: (args) => transferSubscription(client, args),
    accountSubscription: (args) => accountSubscription(client, args),
  };
}
