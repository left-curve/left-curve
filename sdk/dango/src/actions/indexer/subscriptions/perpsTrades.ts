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
};

export type PerpsTradesSubscriptionReturnType = () => void;

/**
 * Subscribes to perps trade events.
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

  const { pairId, ...callbacks } = parameters;

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
  return client.subscribe({ query, variables: { pairId } }, callbacks);
}
