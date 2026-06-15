import type {
  Address,
  CandleIntervals,
  EventFilter,
  IndexedAccountEvent,
  IndexedBlock,
  IndexedEvent,
  IndexedTransferEvent,
  PerpsCandle,
  PerpsPairStats,
  PerpsTrade,
  QueryRequest,
  QueryResponse,
  SubscriptionEvent as DangoSubscriptionEvent,
  Username,
} from "@left-curve/types";
import type { PublicClient } from "@left-curve/sdk";

export type SubscriptionSchema = [
  {
    key: "block";
    params?: undefined;
    listener: (event: Omit<IndexedBlock, "transactions">) => void;
  },
  {
    key: "transfer";
    params: { username: Username; sinceBlockHeight?: number };
    listener: (event: { transfers: IndexedTransferEvent[] }) => void;
  },
  {
    key: "account";
    params: { userIndex: number; sinceBlockHeight?: number };
    listener: (event: { accounts: IndexedAccountEvent[] }) => void;
  },
  {
    key: "events";
    params: { sinceBlockHeight?: number; filter?: EventFilter[] };
    listener: (events: DangoSubscriptionEvent[]) => void;
  },
  {
    key: "eventsByAddresses";
    params: { addresses: Address[]; sinceBlockHeight?: number };
    listener: (events: IndexedEvent[]) => void;
  },
  {
    key: "perpsCandles";
    params: {
      pairId: string;
      interval: CandleIntervals;
    };
    listener: (event: { perpsCandles: PerpsCandle[] }) => void;
  },
  {
    key: "perpsTrades";
    params: {
      pairId: string;
    };
    listener: (event: { perpsTrades: PerpsTrade }) => void;
  },
  {
    key: "submitTx";
    params?: undefined;
    listener: <T>(
      event:
        | { status: "pending" }
        | { status: "success"; data: T; message?: string }
        | { status: "error"; title: string; description: string },
    ) => void;
  },
  {
    key: "queryApp";
    params: {
      request: QueryRequest;
      interval?: number;
      httpInterval?: number;
    };
    listener: (event: { response: QueryResponse; blockHeight: number }) => void;
  },
  {
    key: "allPerpsPairStats";
    params: {
      httpInterval?: number;
    };
    listener: (event: { allPerpsPairStats: PerpsPairStats[] }) => void;
  },
];

export type SubscriptionKey = SubscriptionSchema[number]["key"];

export type GetSubscriptionDef<K extends SubscriptionKey> = Extract<
  SubscriptionSchema[number],
  { key: K }
>;

export type SubscriptionExecutor<K extends SubscriptionKey> = (context: {
  client: PublicClient;
  params: GetSubscriptionDef<K>["params"];
  getListeners: () => Set<GetSubscriptionDef<K>["listener"]>;
  onError?: (error: unknown) => void;
}) => () => void;

export type SubscribeArguments<K extends SubscriptionKey> =
  GetSubscriptionDef<K>["params"] extends undefined
    ? {
        listener: GetSubscriptionDef<K>["listener"];
        params?: undefined;
        onError?: (error: unknown) => void;
      }
    : {
        listener: GetSubscriptionDef<K>["listener"];
        params: GetSubscriptionDef<K>["params"];
        onError?: (error: unknown) => void;
      };

export type SubscriptionEvent<K extends SubscriptionKey> = Parameters<
  GetSubscriptionDef<K>["listener"]
>[0];

export type SubscriptionStore = {
  subscribe: <K extends SubscriptionKey>(key: K, args: SubscribeArguments<K>) => () => void;
  emit: <K extends SubscriptionKey>(
    { key, params }: { key: K; params?: GetSubscriptionDef<K>["params"] },
    event: SubscriptionEvent<K>,
  ) => void;
};
