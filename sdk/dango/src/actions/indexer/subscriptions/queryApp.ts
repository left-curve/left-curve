import type {
  Chain,
  Client,
  Json,
  QueryRequest,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type QueryAppSubscriptionParameters<T = Json> = SubscriptionCallbacks<{
  QueryApp: T;
}> & {
  request: QueryRequest;
  interval?: number;
};

export type QueryAppSubscriptionReturnType = () => void;

/**
 * Subscribes to query app events.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the query app events.
 */
export function queryAppSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: QueryAppSubscriptionParameters,
): QueryAppSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { request, interval, ...callbacks } = parameters;

  const query = /* GraphQL */ `
    subscription QueryAppSubscription (
      $request: GrugQueryInput!
      $interval: Int! = 10
    ) {
      queryApp(request: $request, blockInterval: $interval)
    }
  `;
  return client.subscribe({ query, variables: { request, interval } }, callbacks);
}
