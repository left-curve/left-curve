import type { TxData } from "./cometbft.js";
import type { ChainStatusResponse, QueryRequest, QueryResponse } from "./queries.js";
import type { SimulateRequest } from "./simulate.js";
import type { Tx, TxOutcome, UnsignedTx } from "./tx.js";
import type { Prettify } from "./utils.js";

export type IndexerSchema = [
  {
    Method: "query_app";
    Parameters: {
      query: QueryRequest;
      height: number;
      prove: boolean;
    };
    ReturnType: QueryResponse[keyof QueryRequest];
  },
  {
    Method: "query_status";
    Parameters: undefined;
    ReturnType: ChainStatusResponse;
  },
  {
    Method: "simulate";
    Parameters: {
      tx: SimulateRequest;
      height: number;
      prove: boolean;
    };
    ReturnType: TxOutcome;
  },
  {
    Method: "broadcast";
    Parameters: { tx: Tx | UnsignedTx; mode: "sync" | "async" | "commit" };
    ReturnType: Prettify<TxData & { hash: Uint8Array }>;
  },
  {
    Method: "query";
    Parameters: { document: string; variables: Record<string, unknown> };
    ReturnType: unknown;
  },
];
