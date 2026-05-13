import type { Client, PairStats, SubscriptionCallbacks } from "@left-curve/types";
import { createSubscription } from "@left-curve/utils";
import { getAllPairStats } from "#actions/dex/queries/getAllPairStats.js";

export type AllPairStatsSubscriptionParameters = SubscriptionCallbacks<{
  allPairStats: PairStats[];
}> & {
  /** HTTP polling interval in milliseconds for fallback when WS is unavailable. */
  httpInterval?: number;
};

export type AllPairStatsSubscriptionReturnType = () => void;

/**
 * Subscribes to real-time 24h statistics for all trading pairs.
 * Uses WebSocket when available, falls back to HTTP polling when WS is unavailable.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the all pair stats events.
 */
export function allPairStatsSubscription(
  client: Client,
  parameters: AllPairStatsSubscriptionParameters,
): AllPairStatsSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { httpInterval = 5_000, ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription AllPairStatsSubscription {
      allPairStats {
        baseDenom
        quoteDenom
        currentPrice
        price24HAgo
        volume24H
        priceChange24H
      }
    }
  `;

  return createSubscription<{ allPairStats: PairStats[] }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query },
          {
            next: (data) => listener(data as { allPairStats: PairStats[] }),
            error: callbacks.error,
            complete: callbacks.complete,
          },
        ),
      httpQuery: async () => {
        const allPairStats = await getAllPairStats(client as Client);
        return { allPairStats };
      },
      httpInterval,
      emitter: subscribe.emitter!,
      getStatus: subscribe.getClientStatus!,
      onError: callbacks.error,
    },
    (data) => callbacks.next(data),
  );
}
