import type { Prettify } from "./utils";

export interface ProofOp {
  readonly type: string;
  readonly key: Uint8Array;
  readonly data: Uint8Array;
}

export interface RpcProofOp {
  readonly type: string;
  /** base64 encoded */
  readonly key: string;
  /** base64 encoded */
  readonly data: string;
}

export interface RpcQueryProof {
  readonly ops: readonly RpcProofOp[];
}

export type RpcAbciQueryResponse = {
  /**
   * Base64 encoded
   *
   * This can be null since this is a byte slice and due to
   * https://github.com/tendermint/tendermint/blob/v0.35.7/abci/types/result.go#L53
   */
  readonly key?: string | null;
  /**
   * Base64 encoded
   *
   * This can be null since this is a byte slice and due to
   * https://github.com/tendermint/tendermint/blob/v0.35.7/abci/types/result.go#L53
   */
  readonly value?: string | null;
  readonly proofOps?: RpcQueryProof | null;
  readonly height?: string;
  readonly index?: string;
  readonly code: number;
  readonly codespace?: string;
  readonly log?: string;
  readonly info?: string;
};

export type AbciQueryResponse = {
  readonly key: Uint8Array;
  readonly value: Uint8Array;
  /** proof temporally not supported for abci */
  readonly proof?: null;
  readonly height?: number;
  readonly index?: number;
  readonly code?: number;
  readonly codespace: string;
  readonly log?: string;
  readonly info: string;
};

export type RpcTxData = {
  readonly codespace?: string;
  readonly code?: number;
  readonly log?: string;
  /** base64 encoded */
  readonly data?: string;
  readonly events?: readonly RpcEvent[];
  readonly gas_wanted?: string;
  readonly gas_used?: string;
};

export type RpcEvent = {
  readonly type: string;
  readonly attributes?: readonly RpcEventAttribute[];
};

export type RpcEventAttribute = {
  readonly key: string;
  readonly value?: string;
};

export type RpcBroadcastTxSyncResponse = {
  code: number;
  codespace: string;
  data: string;
  hash: string;
  log: string;
};
