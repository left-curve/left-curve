import type {
  CandleIntervals,
  Chain,
  Client,
  PerpsCandle,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type PerpsCandlesSubscriptionParameters = SubscriptionCallbacks<{
  perpsCandles: PerpsCandle[];
}> & {
  pairId: string;
  interval: CandleIntervals;
};

export type PerpsCandlesSubscriptionReturnType = () => void;

/**
 * Subscribes to perps candle events.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the perps candle events.
 */
export function perpsCandlesSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: PerpsCandlesSubscriptionParameters,
): PerpsCandlesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { pairId, interval, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription PerpsCandlesSubscription (
      $pairId: String!
      $interval: CandleInterval!
    ) {
      perpsCandles(
        pairId: $pairId
        interval: $interval
      ) {
        pairId
        interval
        minBlockHeight
        maxBlockHeight
        open
        high
        low
        close
        volume
        volumeUsd
        timeStart
        timeStartUnix
        timeEnd
        timeEndUnix
      }
    }
  `;
  return client.subscribe({ query, variables: { pairId, interval } }, callbacks);
}
