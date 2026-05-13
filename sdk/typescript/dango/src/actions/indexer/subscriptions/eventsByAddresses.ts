import type { Address, Client, IndexedEvent, SubscriptionCallbacks } from "@left-curve/types";
import { createSubscription } from "@left-curve/utils";

export type EventsByAddressesSubscriptionParameters = SubscriptionCallbacks<{
  eventByAddresses: IndexedEvent[];
}> & {
  addresses: Address[];
  sinceBlockHeight?: number;
};

export type EventsByAddressesSubscriptionReturnType = () => void;

/**
 * Subscribes to all events for a list of addresses.
 * Currently WS-only.
 * @param client The client instance to use for the subscription.
 * @param parameters The parameters for the subscription, including the addresses and callbacks.
 * @returns A function to unsubscribe from the events.
 */
export function eventsByAddressesSubscription(
  client: Client,
  parameters: EventsByAddressesSubscriptionParameters,
): EventsByAddressesSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { addresses, sinceBlockHeight, ...callbacks } = parameters;
  const { subscribe } = client;

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

  return createSubscription<{ eventByAddresses: IndexedEvent[] }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { addresses, sinceBlockHeight } },
          {
            next: (data) => listener(data as { eventByAddresses: IndexedEvent[] }),
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
