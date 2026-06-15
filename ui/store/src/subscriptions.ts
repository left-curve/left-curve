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

function dispatchListeners(
  currentListeners: Set<(...args: any[]) => void> | undefined,
  event: unknown,
  onError?: (error: unknown) => void,
) {
  if (!currentListeners) return;

  for (const listener of currentListeners) {
    try {
      listener(event);
    } catch (error) {
      onError?.(error);
    }
  }
}

export function subscriptionsStore(client: PublicClient, options?: SubscriptionsStoreOptions) {
  const { onError } = options ?? {};
  const activeExecutors: Map<string, () => void> = new Map();
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  const errorListeners = new Map<string, Set<(error: unknown) => void>>();

  const notifyError = (saveKey: string, error: unknown) => {
    onError?.(error);
    const currentErrorListeners = errorListeners.get(saveKey);
    currentErrorListeners?.forEach((listener) => listener(error));
  };

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
        onError: (error: unknown) => notifyError(saveKey, error),
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
    dispatchListeners(listeners.get(saveKey), event, (error) => notifyError(saveKey, error));
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
      dispatchListeners(getListeners(), block, onError);
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
      dispatchListeners(getListeners(), events, onError);
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
      dispatchListeners(getListeners(), eventByAddresses, onError);
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
      dispatchListeners(getListeners(), event, onError);
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
      dispatchListeners(getListeners(), event, onError);
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
      dispatchListeners(getListeners(), event, onError);
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
      dispatchListeners(getListeners(), event, onError);
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
      dispatchListeners(getListeners(), queryApp, onError);
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
      dispatchListeners(getListeners(), event, onError);
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
