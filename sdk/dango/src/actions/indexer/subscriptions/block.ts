import type {
  Chain,
  Client,
  IndexedBlock,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "#types/index.js";

export type BlockSubscriptionParameters = SubscriptionCallbacks<{
  block: Omit<IndexedBlock, "transactions">;
}> & {};

export type BlockSubscriptionReturnType = () => void;

/**
 * Subscribes to block events.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the block events.
 */
export function blockSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: BlockSubscriptionParameters,
): BlockSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const query = /* GraphQL */ `
    subscription {
      block {
        blockHeight
        createdAt
        hash
        transactionsCount
        appHash
      }
    }
  `;

  return client.subscribe({ query, operationName: "block" }, parameters);
}
