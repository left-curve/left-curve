import { createSubscription } from "../../../utils/createSubscription.js";

import type {
  Chain,
  Client,
  IndexedAccountEvent,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type AccountSubscriptionParameters = SubscriptionCallbacks<{
  accounts: IndexedAccountEvent[];
}> & {
  userIndex: number;
  sinceBlockHeight?: number;
};

export type AccountSubscriptionReturnType = () => void;

/**
 * Subscribes to account creation events.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the subscription.
 */
export function accountSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: AccountSubscriptionParameters,
): AccountSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { userIndex, sinceBlockHeight, ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription AccountsSubscription ($userIndex: Int, $sinceBlockHeight: Int) {
      accounts(userIndex: $userIndex, sinceBlockHeight: $sinceBlockHeight) {
        accountIndex
        address
        createdAt
        createdBlockHeight
      }
    }
  `;

  return createSubscription<{ accounts: IndexedAccountEvent[] }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { userIndex, sinceBlockHeight } },
          {
            next: (data) => listener(data as { accounts: IndexedAccountEvent[] }),
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
