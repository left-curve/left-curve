import { createSubscription } from "../../../utils/createSubscription.js";

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
 * Currently WS-only.
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
  const { subscribe } = client;

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

  return createSubscription<{ perpsCandles: PerpsCandle[] }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { pairId, interval } },
          {
            next: (data) => listener(data as { perpsCandles: PerpsCandle[] }),
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
