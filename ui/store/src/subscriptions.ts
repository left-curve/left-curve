import type { PublicClient } from "@left-curve/dango/types";

import type {
  GetSubscriptionDef,
  SubscribeArguments,
  SubscriptionEvent,
  SubscriptionExecutor,
  SubscriptionKey,
} from "./types/subscriptions.js";

export function subscriptionsStore(client: PublicClient) {
  const activeExecutors: Map<string, () => void> = new Map();
  const listeners = new Map<string, Set<(...args: any[]) => void>>();

  const subscribe = <K extends SubscriptionKey>(
    key: K,
    { params, listener }: SubscribeArguments<K>,
  ): (() => void) => {
    const saveKey = JSON.stringify({ key, params });
    if (activeExecutors.has(saveKey)) {
      const currentListeners = listeners.get(saveKey) || new Set();
      currentListeners.add(listener);
      listeners.set(saveKey, currentListeners);
      return () => unsubscribe(saveKey, listener);
    }

    listeners.set(saveKey, new Set([listener]));

    const executor = SubscriptionExecutors[key as keyof typeof SubscriptionExecutors];

    const executorUnsubscribeFn = executor({
      client,
      params,
      getListeners: () => listeners.get(saveKey),
    } as never);

    activeExecutors.set(saveKey, executorUnsubscribeFn);
    return () => unsubscribe(saveKey, listener);
  };

  const unsubscribe = <K extends SubscriptionKey>(
    key: string,
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
    next: ({ eventsByAddresses }) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(eventsByAddresses));
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

const queryAppSubscriptionExecutor: SubscriptionExecutor<"queryApp"> = ({
  client,
  params,
  getListeners,
}) => {
  return client.queryAppSubscription({
    ...params,
    next: (event) => {
      const currentListeners = getListeners();
      currentListeners.forEach((listener) => listener(event));
    },
  });
};

const SubscriptionExecutors = {
  account: accountSubscriptionExecutor,
  block: blockSubscriptionExecutor,
  candles: candlesSubscriptionExecutor,
  eventsByAddresses: eventsByAddressesSubscriptionExecutor,
  submitTx: submitTxSubscriptionExecutor,
  trades: tradesSubscriptionExecutor,
  transfer: transferSubscriptionExecutor,
  queryApp: queryAppSubscriptionExecutor,
};
