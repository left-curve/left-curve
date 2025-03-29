import type {
  CometBftRpcSchema,
  Transport,
  TransportConfig,
  TransportSchema,
} from "../types/index.js";

/**
 * @description Creates an transport intended to be used with a client.
 */
export function createTransport<
  type extends string,
  schema extends TransportSchema = CometBftRpcSchema,
>({
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
