import type { Chain, Client, Signer, SubscriptionCallbacks, Transport } from "#types/index.js";

export type EventsSubscriptionParameters = SubscriptionCallbacks<{
  events: unknown[];
}> & {
  filter: {
    type: string;
    data: {
      path: string[];
      checkMode: "EQUAL" | "CONTAINS";
      value: unknown[];
    }[];
  }[];
};

export type EventsSubscriptionReturnType = () => void;

/**
 * @description Subscribes to events based on specified filters.
 * @param client - The client instance to use for the subscription.
 * @param parameters - The parameters for the subscription, including the filters and callbacks.
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

  const { filter, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription SubscribeEvents($filter: [Filter!]) {
      events(filter: $filter) {
        data
        type
      }
    }
  `;

  return client.subscribe({ query, variables: { filter } }, callbacks);
}
