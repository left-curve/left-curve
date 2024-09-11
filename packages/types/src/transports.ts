import type { AbciQueryResponse } from "./abci";
import type { Chain } from "./chain";
import type { Hex } from "./encoding";
import type { Tx, UnsignedTx } from "./tx";

export type TransportConfig<type extends string = string> = {
  /** The name of the transport. */
  name: string;
  /** The key of the transport. */
  key: string;
  /** The type of the transport. */
  type: type;
  /** Indicates if the transport supports batch queries. */
  batch?: boolean;
};

export type CometQueryFn = (
  path: string,
  data: Uint8Array,
  height?: number,
  prove?: boolean,
) => Promise<AbciQueryResponse>;

export type CometBroadcastFn = (tx: Tx | UnsignedTx) => Promise<Hex>;

export type Transport<type extends string = string> = <chain extends Chain | undefined = Chain>(
  parameters: { chain?: chain | undefined } | undefined,
) => {
  config: TransportConfig<type>;
  query: CometQueryFn;
  broadcast: CometBroadcastFn;
};
