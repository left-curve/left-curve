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

import {
  tradesSubscription,
  type TradesSubscriptionParameters,
  type TradesSubscriptionReturnType,
} from "./subscriptions/trades.js";

import {
  queryAppSubscription,
  type QueryAppSubscriptionParameters,
  type QueryAppSubscriptionReturnType,
} from "./subscriptions/queryApp.js";

import {
  eventsByAddressesSubscription,
  type EventsByAddressesSubscriptionParameters,
  type EventsByAddressesSubscriptionReturnType,
} from "./subscriptions/eventsByAddresses.js";

export type IndexerActions = {
  accountSubscription: (args: AccountSubscriptionParameters) => AccountSubscriptionReturnType;
  blockSubscription: (args: BlockSubscriptionParameters) => BlockSubscriptionReturnType;
  candlesSubscription: (args: CandlesSubscriptionParameters) => CandlesSubscriptionReturnType;
  eventsByAddressesSubscription: (
    args: EventsByAddressesSubscriptionParameters,
  ) => EventsByAddressesSubscriptionReturnType;
  searchTxs: (args: SearchTxsParameters) => SearchTxsReturnType;
  tradesSubscription: (args: TradesSubscriptionParameters) => TradesSubscriptionReturnType;
  transferSubscription: (args: TransferSubscriptionParameters) => TransferSubscriptionReturnType;
  queryAppSubscription: (args: QueryAppSubscriptionParameters) => QueryAppSubscriptionReturnType;
  queryBlock: (args?: QueryBlockParameters) => QueryBlockReturnType;
};

export function indexerActions<transport extends Transport = Transport>(
  client: Client<transport>,
): IndexerActions {
  return {
    blockSubscription: (args) => blockSubscription(client, args),
    accountSubscription: (args) => accountSubscription(client, args),
    candlesSubscription: (args) => candlesSubscription(client, args),
    eventsByAddressesSubscription: (args) => eventsByAddressesSubscription(client, args),
    searchTxs: (args) => searchTxs(client, args),
    tradesSubscription: (args) => tradesSubscription(client, args),
    transferSubscription: (args) => transferSubscription(client, args),
    queryAppSubscription: (args) => queryAppSubscription(client, args),
    queryBlock: (args) => queryBlock(client, args),
  };
}
