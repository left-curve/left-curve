import type {
  Candle,
  CandleIntervals,
  Denom,
  IndexedAccountEvent,
  IndexedBlock,
  IndexedTransferEvent,
  PublicClient,
  QueryRequest,
  Trade,
  Username,
} from "@left-curve/dango/types";

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
    params: { username: Username; sinceBlockHeight?: number };
    listener: (event: { accounts: IndexedAccountEvent[] }) => void;
  },
  {
    key: "candles";
    params: {
      baseDenom: Denom;
      quoteDenom: Denom;
      interval: CandleIntervals;
      laterThan?: Date;
      limit?: number;
    };
    listener: (event: { candles: Candle[] }) => void;
  },
  {
    key: "trades";
    params: {
      baseDenom: Denom;
      quoteDenom: Denom;
    };
    listener: (event: { trades: Trade }) => void;
  },
  {
    key: "submitTx";
    params?: undefined;
    listener: <T>(event: {
      isSubmitting: boolean;
      isSuccess?: boolean;
      message?: string;
      data?: T;
    }) => void;
  },
  {
    key: "queryApp";
    params: {
      request: QueryRequest;
      interval?: number;
    };
    listener: <T>(event: T) => void;
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
}) => () => void;

export type SubscribeArguments<K extends SubscriptionKey> =
  GetSubscriptionDef<K>["params"] extends undefined
    ? { listener: GetSubscriptionDef<K>["listener"]; params?: undefined }
    : { listener: GetSubscriptionDef<K>["listener"]; params: GetSubscriptionDef<K>["params"] };

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
