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

import {
  eventsSubscription,
  type EventsSubscriptionParameters,
  type EventsSubscriptionReturnType,
} from "./subscriptions/events.js";

import {
  perpsCandlesSubscription,
  type PerpsCandlesSubscriptionParameters,
  type PerpsCandlesSubscriptionReturnType,
} from "./subscriptions/perpsCandles.js";

import {
  perpsTradesSubscription,
  type PerpsTradesSubscriptionParameters,
  type PerpsTradesSubscriptionReturnType,
} from "./subscriptions/perpsTrades.js";

import {
  allPairStatsSubscription,
  type AllPairStatsSubscriptionParameters,
  type AllPairStatsSubscriptionReturnType,
} from "./subscriptions/allPairStats.js";

import {
  allPerpsPairStatsSubscription,
  type AllPerpsPairStatsSubscriptionParameters,
  type AllPerpsPairStatsSubscriptionReturnType,
} from "./subscriptions/allPerpsPairStats.js";

export type IndexerActions = {
  accountSubscription: (args: AccountSubscriptionParameters) => AccountSubscriptionReturnType;
  blockSubscription: (args: BlockSubscriptionParameters) => BlockSubscriptionReturnType;
  candlesSubscription: (args: CandlesSubscriptionParameters) => CandlesSubscriptionReturnType;
  eventsSubscription: (args: EventsSubscriptionParameters) => EventsSubscriptionReturnType;
  eventsByAddressesSubscription: (
    args: EventsByAddressesSubscriptionParameters,
  ) => EventsByAddressesSubscriptionReturnType;
  perpsCandlesSubscription: (
    args: PerpsCandlesSubscriptionParameters,
  ) => PerpsCandlesSubscriptionReturnType;
  perpsTradesSubscription: (
    args: PerpsTradesSubscriptionParameters,
  ) => PerpsTradesSubscriptionReturnType;
  allPairStatsSubscription: (
    args: AllPairStatsSubscriptionParameters,
  ) => AllPairStatsSubscriptionReturnType;
  allPerpsPairStatsSubscription: (
    args: AllPerpsPairStatsSubscriptionParameters,
  ) => AllPerpsPairStatsSubscriptionReturnType;
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
    eventsSubscription: (args) => eventsSubscription(client, args),
    eventsByAddressesSubscription: (args) => eventsByAddressesSubscription(client, args),
    perpsCandlesSubscription: (args) => perpsCandlesSubscription(client, args),
    perpsTradesSubscription: (args) => perpsTradesSubscription(client, args),
    allPairStatsSubscription: (args) => allPairStatsSubscription(client, args),
    allPerpsPairStatsSubscription: (args) => allPerpsPairStatsSubscription(client, args),
    searchTxs: (args) => searchTxs(client, args),
    tradesSubscription: (args) => tradesSubscription(client, args),
    transferSubscription: (args) => transferSubscription(client, args),
    queryAppSubscription: (args) => queryAppSubscription(client, args),
    queryBlock: (args) => queryBlock(client, args),
  };
}
