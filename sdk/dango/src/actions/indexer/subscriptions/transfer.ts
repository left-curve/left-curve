import type {
  Chain,
  Client,
  IndexedTransferEvent,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "#types/index.js";

export type TransferSubscriptionParameters = SubscriptionCallbacks<{
  transfers: IndexedTransferEvent[];
}> & {
  username: string;
  sinceBlockHeight?: number;
};

export type TransferSubscriptionReturnType = () => void;

/**
 * @description Subscribes to transfer events for a specific username.
 * @param client - The client instance to use for the subscription.
 * @param parameters - The parameters for the subscription, including the username and callbacks.
 * @returns A function to unsubscribe from the transfer events.
 */
export function transferSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: TransferSubscriptionParameters,
): TransferSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { username, sinceBlockHeight, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription ($username: String, $sinceBlockHeight: Int) {
      transfers(username: $username, sinceBlockHeight: $sinceBlockHeight) {
        id
        txHash
        amount
        denom
        createdAt
        blockHeight
        fromAddress
        toAddress
      }
    }
  `;

  return client.subscribe({ query, variables: { username, sinceBlockHeight } }, callbacks);
}
