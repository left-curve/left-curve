import type {
  Chain,
  Client,
  IndexedTransferEvent,
  OneRequired,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "#types/index.js";

export type TransferSubscriptionParameters = SubscriptionCallbacks<
  OneRequired<
    { sentTransfers: IndexedTransferEvent[]; receivedTransfers: IndexedTransferEvent[] },
    "sentTransfers",
    "receivedTransfers"
  >
> & {
  address: string;
};

export type TransferSubscriptionReturnType = () => void;

/**
 * @description Subscribes to transfer events for a specific address.
 * @param client - The client instance to use for the subscription.
 * @param parameters - The parameters for the subscription, including the address and callbacks.
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

  const { address, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription ($address: String) {
      sentTransfers: transfers(fromAddress: $address) {
        fromAddress
        toAddress
        createdAt
        blockHeight
        amount
        denom
      }
      receivedTransfers: transfers(toAddress: $address) {
        fromAddress
        toAddress
        createdAt
        blockHeight
        amount
        denom
      }
    }
  `;

  return client.subscribe({ query, variables: { address } }, callbacks);
}
