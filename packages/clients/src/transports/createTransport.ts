import type { CometBroadcastFn, CometQueryFn, Transport, TransportConfig } from "@leftcurve/types";

/**
 * @description Creates an transport intended to be used with a client.
 */
export function createTransport<type extends string>(
  { key, name, type }: TransportConfig<type>,
  { query, broadcast }: { query: CometQueryFn; broadcast: CometBroadcastFn },
): ReturnType<Transport<type>> {
  return {
    config: {
      key,
      name,
      type,
    },
    query,
    broadcast,
  };
}
