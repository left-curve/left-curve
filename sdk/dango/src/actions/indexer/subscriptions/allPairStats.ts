import type {
  Chain,
  Client,
  PairStats,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type AllPairStatsSubscriptionParameters = SubscriptionCallbacks<{
  allPairStats: PairStats[];
}>;

export type AllPairStatsSubscriptionReturnType = () => void;

/**
 * Subscribes to real-time 24h statistics for all trading pairs.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the all pair stats events.
 */
export function allPairStatsSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: AllPairStatsSubscriptionParameters,
): AllPairStatsSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

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

  return client.subscribe({ query }, parameters);
}
