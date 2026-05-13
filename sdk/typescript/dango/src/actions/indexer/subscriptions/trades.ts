import type { Client, Denom, SubscriptionCallbacks, Trade } from "@left-curve/types";
import { createSubscription } from "@left-curve/utils";

export type TradesSubscriptionParameters = SubscriptionCallbacks<{
  trades: Trade;
}> & {
  baseDenom: Denom;
  quoteDenom: Denom;
  /** HTTP polling interval in milliseconds for fallback when WS is unavailable. */
  httpInterval?: number;
};

export type TradesSubscriptionReturnType = () => void;

/**
 * Subscribes to trade events.
 * Currently WS-only;
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the trade events.
 */
export function tradesSubscription(
  client: Client,
  parameters: TradesSubscriptionParameters,
): TradesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { baseDenom, quoteDenom, httpInterval = 3_000, ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription TradesSubscription($baseDenom: String!, $quoteDenom: String!) {
      trades(baseDenom: $baseDenom, quoteDenom: $quoteDenom) {
        addr
        quoteDenom
        baseDenom
        direction
        blockHeight
        createdAt
        filledBase
        filledQuote
        refundBase
        refundQuote
        feeBase
        feeQuote
        clearingPrice
      }
    }
  `;

  return createSubscription<{ trades: Trade }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { baseDenom, quoteDenom } },
          {
            next: (data) => listener(data as { trades: Trade }),
            error: callbacks.error,
            complete: callbacks.complete,
          },
        ),
      httpQuery: undefined,
      httpInterval,
      emitter: subscribe.emitter!,
      getStatus: subscribe.getClientStatus!,
      onError: callbacks.error,
    },
    (data) => callbacks.next(data),
  );
}
