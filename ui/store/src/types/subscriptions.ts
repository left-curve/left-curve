import type {
  Address,
  IndexedBlock,
  IndexedTransferEvent,
  PublicClient,
} from "@left-curve/dango/types";

export type SubscriptionSchema = [
  {
    key: "block";
    params?: undefined;
    listener: (event: Omit<IndexedBlock, "transactions">) => void;
  },
  {
    key: "transfer";
    params: { address: Address };
    listener: (event: IndexedTransferEvent) => void;
  },
  {
    key: "submitTx";
    params?: undefined;
    listener: (event: {
      isSubmitting: boolean;
      txResult?: { hasSucceeded: boolean; message: string };
    }) => void;
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
  emit: <K extends SubscriptionKey>(key: K, event: SubscriptionEvent<K>) => void;
};
