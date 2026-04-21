import { createSubscription } from "../../../utils/createSubscription.js";
import { queryApp } from "../../app/queries/queryApp.js";

import type {
  Chain,
  Client,
  QueryRequest,
  QueryResponse,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type QueryAppSubscriptionParameters = SubscriptionCallbacks<{
  queryApp: {
    response: QueryResponse;
    blockHeight: number;
  };
}> & {
  request: QueryRequest;
  interval?: number;
  /** HTTP polling interval in milliseconds for fallback when WS is unavailable. */
  httpInterval?: number;
};

export type QueryAppSubscriptionReturnType = () => void;

/**
 * Subscribes to query app events.
 * Uses WebSocket when available, falls back to HTTP polling when WS is unavailable.
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

  const { request, interval, httpInterval = 5_000, ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription QueryAppSubscription (
      $request: GrugQueryInput!
      $interval: Int! = 10
    ) {
      queryApp(request: $request, blockInterval: $interval) {
        response
        blockHeight
      }
    }
  `;

  return createSubscription<{ queryApp: { response: QueryResponse; blockHeight: number } }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query, variables: { request, interval } },
          {
            next: (data) =>
              listener(data as { queryApp: { response: QueryResponse; blockHeight: number } }),
            error: callbacks.error,
            complete: callbacks.complete,
          },
        ),
      httpQuery: async () => {
        const response = await queryApp(client, { query: request });
        return { queryApp: { response, blockHeight: 0 } };
      },
      httpInterval,
      emitter: subscribe.emitter!,
      getStatus: subscribe.getClientStatus!,
      onError: callbacks.error,
    },
    (data) => callbacks.next(data),
  );
}
