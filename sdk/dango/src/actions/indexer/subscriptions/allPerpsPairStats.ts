import { createSubscription } from "../../../utils/createSubscription.js";
import { getAllPerpsPairStats } from "../../perps/queries/getAllPerpsPairStats.js";

import type {
  Chain,
  Client,
  PerpsPairStats,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type AllPerpsPairStatsSubscriptionParameters = SubscriptionCallbacks<{
  allPerpsPairStats: PerpsPairStats[];
}> & {
  /** HTTP polling interval in milliseconds for fallback when WS is unavailable. */
  httpInterval?: number;
};

export type AllPerpsPairStatsSubscriptionReturnType = () => void;

/**
 * Subscribes to real-time 24h statistics for all perps trading pairs.
 * Uses WebSocket when available, falls back to HTTP polling when WS is unavailable.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the all perps pair stats events.
 */
export function allPerpsPairStatsSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: AllPerpsPairStatsSubscriptionParameters,
): AllPerpsPairStatsSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { httpInterval = 5_000, ...callbacks } = parameters;
  const { subscribe } = client;
  const { polling, batch } = client.transport;

  const query = /* GraphQL */ `
    subscription AllPerpsPairStatsSubscription {
      allPerpsPairStats {
        pairId
        currentPrice
        price24HAgo
        volume24H
        priceChange24H
      }
    }
  `;

  return createSubscription<{ allPerpsPairStats: PerpsPairStats[] }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query },
          {
            next: (data) => listener(data as { allPerpsPairStats: PerpsPairStats[] }),
            error: callbacks.error,
            complete: callbacks.complete,
          },
        ),
      httpQuery: async () => {
        const allPerpsPairStats = await getAllPerpsPairStats(client as Client<Transport>);
        return { allPerpsPairStats };
      },
      httpInterval,
      emitter: subscribe.emitter!,
      getStatus: subscribe.getClientStatus!,
      onError: callbacks.error,
      polling,
      batch,
    },
    (data) => callbacks.next(data),
  );
}
