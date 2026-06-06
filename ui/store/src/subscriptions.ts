import type { PublicClient } from "@left-curve/sdk";

import type {
  GetSubscriptionDef,
  SubscribeArguments,
  SubscriptionEvent,
  SubscriptionExecutor,
  SubscriptionKey,
} from "./types/subscriptions.js";

export type SubscriptionsStoreOptions = {
  onError?: (error: unknown) => void;
};

export function subscriptionsStore(client: PublicClient, options?: SubscriptionsStoreOptions) {
  const { onError } = options ?? {};
  const activeExecutors: Map<string, () => void> = new Map();
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  const errorListeners = new Map<string, Set<(error: unknown) => void>>();

  const subscribe = <K extends SubscriptionKey>(
    key: K,
    { params, listener, onError: listenerOnError }: SubscribeArguments<K>,
  ): (() => void) => {
    const saveKey = JSON.stringify({ key, params });
    if (listenerOnError) {
      const currentErrorListeners = errorListeners.get(saveKey) || new Set();
      currentErrorListeners.add(listenerOnError);
      errorListeners.set(saveKey, currentErrorListeners);
    }

    if (activeExecutors.has(saveKey)) {
      const currentListeners = listeners.get(saveKey) || new Set();
      currentListeners.add(listener);
      listeners.set(saveKey, currentListeners);
      return () => unsubscribe(saveKey, listener, listenerOnError);
    }

    listeners.set(saveKey, new Set([listener]));

    const executor = SubscriptionExecutors[key as keyof typeof SubscriptionExecutors];

    try {
      const executorUnsubscribeFn = executor({
        client,
        params,
        getListeners: () => listeners.get(saveKey),
        onError: (error: unknown) => {
          onError?.(error);
          const currentErrorListeners = errorListeners.get(saveKey);
          currentErrorListeners?.forEach((listener) => listener(error));
        },
      } as never);

      activeExecutors.set(saveKey, executorUnsubscribeFn);
    } catch (error) {
      listeners.delete(saveKey);
      errorListeners.delete(saveKey);
      throw error;
    }

    return () => unsubscribe(saveKey, listener, listenerOnError);
  };

  const unsubscribe = <K extends SubscriptionKey>(
    key: string,
    listener: GetSubscriptionDef<K>["listener"],
    listenerOnError?: (error: unknown) => void,
  ): void => {
    if (!activeExecutors.has(key)) return;

    const currentListeners = listeners.get(key);
    const currentErrorListeners = errorListeners.get(key);
    if (listenerOnError) {
      currentErrorListeners?.delete(listenerOnError);
      if (currentErrorListeners?.size === 0) errorListeners.delete(key);
    }
    if (currentListeners) {
      currentListeners.delete(listener);
      if (currentListeners.size === 0) {
        activeExecutors.get(key)?.();
        activeExecutors.delete(key);
        listeners.delete(key);
        errorListeners.delete(key);
      }
    }
  };

  const emit = <K extends SubscriptionKey>(
    { key, params }: { key: K; params?: GetSubscriptionDef<K>["params"] },
    event: SubscriptionEvent<K>,
  ): void => {
    const saveKey = JSON.stringify({ key, params });
    const currentListeners = listeners.get(saveKey);
    if (currentListeners) {
      currentListeners.forEach((listener) => listener(event));
    }
  };

  return {
    subscribe,
    emit,
  };
}

const blockSubscriptionExecutor: SubscriptionExecutor<"block"> = ({
  client,
  getListeners,
  onError,
}) => {
  const unsubscribe = client.blockSubscription({
    next: ({ block }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(block));
    },
    error: onError,
  });
  return unsubscribe;
};

const eventsSubscriptionExecutor: SubscriptionExecutor<"events"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.eventsSubscription({
    ...params,
    next: ({ events }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(events));
    },
    error: onError,
  });
};

const eventsByAddressesSubscriptionExecutor: SubscriptionExecutor<"eventsByAddresses"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.eventsByAddressesSubscription({
    ...params,
    next: ({ eventByAddresses }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(eventByAddresses));
    },
    error: onError,
  });
};

const transferSubscriptionExecutor: SubscriptionExecutor<"transfer"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.transferSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
    error: onError,
  });
};

const accountSubscriptionExecutor: SubscriptionExecutor<"account"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.accountSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => {
        listener(event);
      });
    },
    error: onError,
  });
};

const perpsCandlesSubscriptionExecutor: SubscriptionExecutor<"perpsCandles"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.perpsCandlesSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
    error: onError,
  });
};

const perpsTradesSubscriptionExecutor: SubscriptionExecutor<"perpsTrades"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.perpsTradesSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
    error: onError,
  });
};

const submitTxSubscriptionExecutor: SubscriptionExecutor<"submitTx"> = () => {
  // This execute is a noop function for submitTx subscription.
  return () => {};
};

const queryAppSubscriptionExecutor: SubscriptionExecutor<"queryApp"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.queryAppSubscription({
    ...params,
    next: ({ queryApp }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(queryApp));
    },
    error: onError,
  });
};

const allPerpsPairStatsSubscriptionExecutor: SubscriptionExecutor<"allPerpsPairStats"> = ({
  client,
  params,
  getListeners,
  onError,
}) => {
  return client.allPerpsPairStatsSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
    error: onError,
  });
};

const SubscriptionExecutors = {
  account: accountSubscriptionExecutor,
  block: blockSubscriptionExecutor,
  events: eventsSubscriptionExecutor,
  eventsByAddresses: eventsByAddressesSubscriptionExecutor,
  perpsCandles: perpsCandlesSubscriptionExecutor,
  perpsTrades: perpsTradesSubscriptionExecutor,
  submitTx: submitTxSubscriptionExecutor,
  transfer: transferSubscriptionExecutor,
  queryApp: queryAppSubscriptionExecutor,
  allPerpsPairStats: allPerpsPairStatsSubscriptionExecutor,
};
