import type {
  Chain,
  Client,
  EventFilter,
  Signer,
  SubscriptionCallbacks,
  SubscriptionEvent,
  Transport,
} from "../../../types/index.js";

export type EventsSubscriptionParameters = SubscriptionCallbacks<{
  events: SubscriptionEvent[];
}> & {
  sinceBlockHeight?: number;
  filter?: EventFilter[];
};

export type EventsSubscriptionReturnType = () => void;

/**
 * @description Subscribes to events with flexible filtering by type and data paths.
 * @param client - The client instance to use for the subscription.
 * @param parameters - The parameters for the subscription, including optional filter and callbacks.
 * @returns A function to unsubscribe from the events.
 */
export function eventsSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: EventsSubscriptionParameters,
): EventsSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { sinceBlockHeight, filter, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription SubscribeEvents(
      $sinceBlockHeight: Int
      $filter: [Filter!]
    ) {
      events(
        sinceBlockHeight: $sinceBlockHeight
        filter: $filter
      ) {
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
      }
    }
  `;

  return client.subscribe({ query, variables: { sinceBlockHeight, filter } }, callbacks);
}
