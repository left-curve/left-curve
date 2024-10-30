import type { Chain } from "./chain.js";
import type { CometBftRpcSchema } from "./cometbft.js";
import type { RequestFn, RpcSchema } from "./rpc.js";

export type TransportConfig<
  type extends string = string,
  rpcSchema extends RpcSchema = CometBftRpcSchema,
> = {
  /** The name of the transport. */
  name: string;
  /** The key of the transport. */
  key: string;
  /** The type of the transport. */
  type: type;
  /** Indicates if the transport supports batch queries. */
  batch?: boolean;
  request: RequestFn<rpcSchema>;
};

export type Transport<
  type extends string = string,
  rpcSchema extends RpcSchema = CometBftRpcSchema,
> = <chain extends Chain | undefined = Chain>(
  parameters: { chain?: chain | undefined } | undefined,
) => {
  config: TransportConfig<type, rpcSchema>;
  request: RequestFn<rpcSchema>;
};
