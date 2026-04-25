import { createSubscription } from "../../../utils/createSubscription.js";

import type {
  Chain,
  Client,
  PerpsTrade,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type PerpsTradesSubscriptionParameters = SubscriptionCallbacks<{
  perpsTrades: PerpsTrade;
}> & {
  pairId: string;
  /** HTTP polling interval in milliseconds for fallback when WS is unavailable. */
  httpInterval?: number;
};

export type PerpsTradesSubscriptionReturnType = () => void;

/**
 * Subscribes to perps trade events.
 * Currently WS-only.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the perps trade events.
 */
export function perpsTradesSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: PerpsTradesSubscriptionParameters,
): PerpsTradesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { pairId, httpInterval = 3_000, ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription PerpsTradesSubscription($pairId: String!) {
      perpsTrades(pairId: $pairId) {
        orderId
        pairId
        user
        fillPrice
        fillSize
        closingSize
        openingSize
        realizedPnl
        fee
        createdAt
        blockHeight
        tradeIdx
      }
    }
  `;

  return createSubscription<{ perpsTrades: PerpsTrade }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { pairId } },
          {
            next: (data) => listener(data as { perpsTrades: PerpsTrade }),
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
