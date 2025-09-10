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
};

export type TradesSubscriptionReturnType = () => void;

/**
 * Subscribes to trade events.
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

  const { baseDenom, quoteDenom, ...callbacks } = parameters;

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
  return client.subscribe({ query, variables: { baseDenom, quoteDenom } }, callbacks);
}
