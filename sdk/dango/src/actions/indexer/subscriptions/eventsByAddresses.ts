import type {
  Address,
  Chain,
  Client,
  IndexedEvent,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "#types/index.js";

export type EventsByAddressesSubscriptionParameters = SubscriptionCallbacks<{
  eventByAddresses: IndexedEvent[];
}> & {
  addresses: Address[];
  sinceBlockHeight?: number;
};

export type EventsByAddressesSubscriptionReturnType = () => void;

/**
 * @description Subscribes to all events for a list of addresses.
 * @param client - The client instance to use for the subscription.
 * @param parameters - The parameters for the subscription, including the addresses and callbacks.
 * @returns A function to unsubscribe from the events.
 */
export function eventsByAddressesSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: EventsByAddressesSubscriptionParameters,
): EventsByAddressesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { addresses, sinceBlockHeight, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription EventsByAddressesSubscription(
      $addresses: [String!]!
      $sinceBlockHeight: Int
    ) {
      eventByAddresses(
        addresses: $addresses
        sinceBlockHeight: $sinceBlockHeight
      ) {
        id
        parentId
        transactionId
        messageId
        type
        method
        eventStatus
        commitmentStatus
        transactionType
        transactionIdx
        messageIdx
        eventIdx
        data
        blockHeight
        createdAt
        transaction {
          hash
        }
      }
    }
  `;

  return client.subscribe({ query, variables: { addresses, sinceBlockHeight } }, callbacks);
}
