import { createLiveResource } from "../live/createLiveResource.js";
import { useLiveResource } from "../live/useLiveResource.js";
import { useConfig } from "./useConfig.js";

import type { PerpsTrade } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

export type LivePerpsTradesSnapshot = LiveResourceSnapshot & {
  trades: PerpsTrade[];
  currentPrice: string | null;
  previousPrice: string | null;
};

export type UseLivePerpsTradesParameters = {
  perpsPairId?: string;
  enabled?: boolean;
};

type LivePerpsTradesResourceParams = {
  chainId: Config["chain"]["id"];
  perpsPairId: string;
  subscriptions: Config["subscriptions"];
};

const initialLivePerpsTradesSnapshot: LivePerpsTradesSnapshot = {
  status: "idle",
  error: null,
  trades: [],
  currentPrice: null,
  previousPrice: null,
};

const livePerpsTradesResource = createLiveResource<
  LivePerpsTradesResourceParams,
  LivePerpsTradesSnapshot
>({
  name: "livePerpsTrades",
  getKey: ({ chainId, perpsPairId }) => `livePerpsTrades:${chainId}:${perpsPairId}`,
  getInitialSnapshot: () => initialLivePerpsTradesSnapshot,
  equal: (previous, next) =>
    previous.status === next.status &&
    previous.error === next.error &&
    previous.currentPrice === next.currentPrice &&
    previous.previousPrice === next.previousPrice &&
    previous.trades === next.trades,
  start: ({ perpsPairId, subscriptions }, { emit, error }) => {
    let snapshot = initialLivePerpsTradesSnapshot;
    let tradesBuffer: PerpsTrade[] = [];
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    const processBuffer = () => {
      if (tradesBuffer.length === 0) {
        debounceTimer = null;
        return;
      }

      const trades = [...tradesBuffer, ...snapshot.trades].slice(0, 50);
      const currentPrice = trades[0]?.fillPrice ?? null;
      snapshot = {
        status: "ready",
        error: null,
        trades,
        previousPrice: snapshot.currentPrice,
        currentPrice,
      };
      tradesBuffer = [];
      debounceTimer = null;
      emit(snapshot);
    };

    const unsubscribe = subscriptions.subscribe("perpsTrades", {
      params: { pairId: perpsPairId },
      listener: ({ perpsTrades: trade }) => {
        if (trade.isMaker === true) return;
        tradesBuffer.unshift(trade);
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(processBuffer, 500);
      },
      onError: error,
    });

    return () => {
      unsubscribe();
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  },
});

export function useLivePerpsTrades<Selection>(
  selector: (snapshot: LivePerpsTradesSnapshot) => Selection,
  parameters: UseLivePerpsTradesParameters,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { perpsPairId, enabled = true } = parameters;
  const config = useConfig();

  return useLiveResource({
    resource: livePerpsTradesResource,
    params: {
      chainId: config.chain.id,
      perpsPairId: perpsPairId ?? "",
      subscriptions: config.subscriptions,
    },
    enabled: enabled && !!perpsPairId,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
