import type {
  Candle,
  CandleIntervals,
  Client,
  Denom,
  SubscriptionCallbacks,
} from "@left-curve/types";
import { createSubscription } from "@left-curve/utils";

export type CandlesSubscriptionParameters = SubscriptionCallbacks<{
  candles: Candle[];
}> & {
  baseDenom: Denom;
  quoteDenom: Denom;
  interval: CandleIntervals;
};

export type CandlesSubscriptionReturnType = () => void;

/**
 * Subscribes to candle events.
 * Currently WS-only.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the candle events.
 */
export function candlesSubscription(
  client: Client,
  parameters: CandlesSubscriptionParameters,
): CandlesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { baseDenom, quoteDenom, interval, ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription CandlesSubscription (
      $baseDenom: String!
      $quoteDenom: String!
      $interval: CandleInterval!
    ) {
      candles(
        baseDenom: $baseDenom
        quoteDenom: $quoteDenom
        interval: $interval
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

  return createSubscription<{ candles: Candle[] }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { baseDenom, quoteDenom, interval } },
          {
            next: (data) => listener(data as { candles: Candle[] }),
            error: callbacks.error,
            complete: callbacks.complete,
          },
        ),
      httpQuery: undefined,
      httpInterval: 5_000,
      emitter: subscribe.emitter!,
      getStatus: subscribe.getClientStatus!,
      onError: callbacks.error,
    },
    (data) => callbacks.next(data),
  );
}
