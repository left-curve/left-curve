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
}>;

export type AllPerpsPairStatsSubscriptionReturnType = () => void;

/**
 * Subscribes to real-time 24h statistics for all perps trading pairs.
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

  return client.subscribe({ query }, parameters);
}
