import type {
  Candle,
  CandleIntervals,
  Chain,
  Client,
  Denom,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "#types/index.js";

export type CandlesSubscriptionParameters = SubscriptionCallbacks<{
  candles: Candle[];
}> & {
  baseDenom: Denom;
  quoteDenom: Denom;
  interval: CandleIntervals;
  laterThan?: Date;
  limit?: number;
};

export type CandlesSubscriptionReturnType = () => void;

/**
 * Subscribes to candle events.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the candle events.
 */
export function candlesSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: CandlesSubscriptionParameters,
): CandlesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { baseDenom, quoteDenom, interval, laterThan, limit, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription (
      $baseDenom: String!
      $quoteDenom: String!
      $interval: CandleIntervals!
      $laterThan: DateTime
      $limit: Int
    ) {
      candles(
        baseDenom: $baseDenom
        quoteDenom: $quoteDenom
        interval: $interval
        laterThan: $laterThan
        limit: $limit
      ) {
        quoteDenom
        baseDenom
        interval
        blockHeight
        open
        high
        low
        close
        volumeBase
        volumeQuote
        timeStart
        timeStartUnix
        timeEnd
        timeEndUnix
      }
    }
  `;
  return client.subscribe(
    { query, variables: { baseDenom, quoteDenom, interval, laterThan, limit } },
    callbacks,
  );
}
