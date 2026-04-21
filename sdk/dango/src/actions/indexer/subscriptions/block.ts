import { createSubscription } from "../../../utils/createSubscription.js";

import type {
  Chain,
  Client,
  IndexedBlock,
  Signer,
  SubscriptionCallbacks,
  Transport,
} from "../../../types/index.js";

export type BlockSubscriptionParameters = SubscriptionCallbacks<{
  block: Omit<IndexedBlock, "transactions">;
}> & {};

export type BlockSubscriptionReturnType = () => void;

/**
 * Subscribes to block events.
 * Currently WS-only.
 * @param client The Dango client.
 * @param parameters The parameters for the subscription.
 * @returns A function to unsubscribe from the block events.
 */
export function blockSubscription<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: BlockSubscriptionParameters,
): BlockSubscriptionReturnType {
  if (!client.subscribe) throw new Error("error: client does not support subscriptions");

  const { ...callbacks } = parameters;
  const { subscribe } = client;

  const query = /* GraphQL */ `
    subscription BlockSubscription {
      block {
        blockHeight
        createdAt
        hash
        transactionsCount
        appHash
      }
    }
  `;

  return createSubscription<{ block: Omit<IndexedBlock, "transactions"> }>(
    {
      wsSubscribe: (listener) =>
        subscribe(
          { query },
          {
            next: (data) => listener(data as { block: Omit<IndexedBlock, "transactions"> }),
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
