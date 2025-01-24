import type { CometBftRpcSchema, RpcSchema, Transport, TransportConfig } from "../types/index.js";

/**
 * @description Creates an transport intended to be used with a client.
 */
export function createTransport<type extends string, schema extends RpcSchema = CometBftRpcSchema>({
  key,
  name,
  type,
  request,
}: TransportConfig<type, schema>): ReturnType<Transport<type, schema>> {
  return {
    config: {
      key,
      name,
      type,
      request,
    },
    request,
  };
}
