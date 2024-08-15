export interface ProofOp {
  readonly type: string;
  readonly key: Uint8Array;
  readonly data: Uint8Array;
}
export interface QueryProof {
  readonly ops: readonly ProofOp[];
}
export interface AbciQueryResponse {
  readonly key: Uint8Array;
  readonly value: Uint8Array;
  readonly proof?: QueryProof;
  readonly height?: number;
  readonly index?: number;
  readonly code?: number;
  readonly codespace: string;
  readonly log?: string;
  readonly info: string;
}
