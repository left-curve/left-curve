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
 * Subscribes to account creation events
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

  const query = /* GraphQL */ `
    subscription AccountsSubscription ($userIndex: Int, $sinceBlockHeight: Int) {
      accounts(userIndex: $userIndex, sinceBlockHeight: $sinceBlockHeight) {
        accountIndex
        address
        accountType
        createdAt
        createdBlockHeight
      }
    }
  `;

  return client.subscribe({ query, variables: { userIndex, sinceBlockHeight } }, callbacks);
}
