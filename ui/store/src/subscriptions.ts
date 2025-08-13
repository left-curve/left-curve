import type { PublicClient } from "@left-curve/dango/types";

import type {
  GetSubscriptionDef,
  SubscribeArguments,
  SubscriptionEvent,
  SubscriptionExecutor,
  SubscriptionKey,
} from "./types/subscriptions.js";

export function subscriptionsStore(client: PublicClient) {
  const activeExecutors: Map<SubscriptionKey, () => void> = new Map();
  const listeners = new Map<SubscriptionKey, Set<(...args: any[]) => void>>();

  const subscribe = <K extends SubscriptionKey>(
    key: K,
    { params, listener }: SubscribeArguments<K>,
  ): (() => void) => {
    if (activeExecutors.has(key)) {
      const currentListeners = listeners.get(key) || new Set();
      currentListeners.add(listener);
      listeners.set(key, currentListeners);
      return () => unsubscribe(key, listener);
    }

    listeners.set(key, new Set([listener]));

    const executor = SubscriptionExecutors[key as keyof typeof SubscriptionExecutors];

    const executorUnsubscribeFn = executor({
      client,
      params,
      getListeners: () => listeners.get(key),
    } as never);

    activeExecutors.set(key, executorUnsubscribeFn);
    return () => unsubscribe(key, listener);
  };

  const unsubscribe = <K extends SubscriptionKey>(
    key: K,
    listener: GetSubscriptionDef<K>["listener"],
  ): void => {
    if (!activeExecutors.has(key)) return;

    const currentListeners = listeners.get(key);
    if (currentListeners) {
      currentListeners.delete(listener);
      if (currentListeners.size === 0) {
        activeExecutors.get(key)?.();
        activeExecutors.delete(key);
        listeners.delete(key);
      }
    }
  };

  const emit = <K extends SubscriptionKey>(key: K, event: SubscriptionEvent<K>): void => {
    const currentListeners = listeners.get(key);
    if (currentListeners) {
      currentListeners.forEach((listener) => listener(event));
    }
  };

  return {
    subscribe,
    emit,
  };
}

const blockSubscriptionExecutor: SubscriptionExecutor<"block"> = ({ client, getListeners }) => {
  const unsubscribe = client.blockSubscription({
    next: ({ block }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(block));
    },
  });
  return unsubscribe;
};

const eventsByAddressesSubscriptionExecutor: SubscriptionExecutor<"eventsByAddresses"> = ({
  client,
  params,
  getListeners,
}) => {
  return client.eventsByAddressesSubscription({
    ...params,
    next: ({ eventByAddresses }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(eventByAddresses));
    },
  });
};

const transferSubscriptionExecutor: SubscriptionExecutor<"transfer"> = ({
  client,
  params,
  getListeners,
}) => {
  return client.transferSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
  });
};

const accountSubscriptionExecutor: SubscriptionExecutor<"account"> = ({
  client,
  params,
  getListeners,
}) => {
  return client.accountSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => {
        listener(event);
      });
    },
  });
};

const candlesSubscriptionExecutor: SubscriptionExecutor<"candles"> = ({
  client,
  params,
  getListeners,
}) => {
  return client.candlesSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
  });
};

const tradesSubscriptionExecutor: SubscriptionExecutor<"trades"> = ({
  client,
  params,
  getListeners,
}) => {
  return client.tradesSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
  });
};

const submitTxSubscriptionExecutor: SubscriptionExecutor<"submitTx"> = () => {
  // This execute is a noop function for submitTx subscription.
  return () => {};
};

const SubscriptionExecutors = {
  account: accountSubscriptionExecutor,
  block: blockSubscriptionExecutor,
  candles: candlesSubscriptionExecutor,
  eventsByAddresses: eventsByAddressesSubscriptionExecutor,
  submitTx: submitTxSubscriptionExecutor,
  trades: tradesSubscriptionExecutor,
  transfer: transferSubscriptionExecutor,
};
