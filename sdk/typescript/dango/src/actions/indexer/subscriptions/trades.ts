import { createSubscription } from "../../../utils/createSubscription.js";

import type {
  Chain,
  Client,
  Denom,
  Signer,
  SubscriptionCallbacks,
  Trade,
  Transport,
} from "../../../types/index.js";

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
export function tradesSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
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
