import type { Base64, Hex } from "./encoding.js";

export type QueryAbciResponse = {
  readonly key?: Base64 | null;
  readonly value?: Base64 | null;
  readonly proofOps?: { ops: ProofOp[] } | null;
  readonly height?: string;
  readonly index?: string;
  readonly code?: number;
  readonly codespace?: string;
  readonly log?: string;
  readonly info?: string;
};

export type TxResponse = {
  readonly tx: Base64;
  readonly tx_result: TxData;
  readonly height: string;
  readonly index: number;
  readonly hash: Hex;
  readonly proof?: TxProof;
};

export type TxProof = {
  readonly data: Base64;
  readonly root_hash: Hex;
  readonly proof: {
    readonly total: string;
    readonly index: string;
    readonly leaf_hash: Base64;
    readonly aunts: Base64[];
  };
};

export type TxData = {
  readonly codespace?: string;
  readonly code?: number;
  readonly log?: string;
  readonly data?: Base64;
  readonly events?: TxEvent[];
  readonly gas_wanted?: string;
  readonly gas_used?: string;
};

export type TxEvent = {
  readonly type: string;
  readonly attributes?: TxEventAttribute[];
};

export type TxEventAttribute = {
  readonly key: string;
  readonly value?: string;
};

export type ProofOp = {
  readonly type: string;
  readonly key: Base64;
  readonly data: Base64;
};
