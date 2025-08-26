import type {
  Chain,
  Client,
  IndexedAccountEvent,
  Signer,
  SubscriptionCallbacks,
  Transport,
  Username,
} from "../../../types/index.js";

export type AccountSubscriptionParameters = SubscriptionCallbacks<{
  accounts: IndexedAccountEvent[];
}> & {
  username: Username;
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

  const { username, sinceBlockHeight, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription AccountsSubscription ($username: String, $sinceBlockHeight: Int) {
      accounts(username: $username, sinceBlockHeight: $sinceBlockHeight) {
        accountIndex
        address
        accountType
        createdAt
        createdBlockHeight
      }
    }
  `;

  return client.subscribe({ query, variables: { username, sinceBlockHeight } }, callbacks);
}
