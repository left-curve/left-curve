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
  type CandlesSubscriptionParameters,
  type CandlesSubscriptionReturnType,
  candlesSubscription,
} from "./subscriptions/candles.js";

import {
  type TransferSubscriptionParameters,
  type TransferSubscriptionReturnType,
  transferSubscription,
} from "./subscriptions/transfer.js";

export type IndexerActions = {
  accountSubscription: (args: AccountSubscriptionParameters) => AccountSubscriptionReturnType;
  blockSubscription: (args: BlockSubscriptionParameters) => BlockSubscriptionReturnType;
  candlesSubscription: (args: CandlesSubscriptionParameters) => CandlesSubscriptionReturnType;
  searchTxs: (args: SearchTxsParameters) => SearchTxsReturnType;
  transferSubscription: (args: TransferSubscriptionParameters) => TransferSubscriptionReturnType;
  queryBlock: (args?: QueryBlockParameters) => QueryBlockReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: Client<transport>,
): IndexerActions {
  return {
    blockSubscription: (args) => blockSubscription(client, args),
    accountSubscription: (args) => accountSubscription(client, args),
    candlesSubscription: (args) => candlesSubscription(client, args),
    searchTxs: (args) => searchTxs(client, args),
    transferSubscription: (args) => transferSubscription(client, args),
    queryBlock: (args) => queryBlock(client, args),
  };
}
