import type { Json } from "./encoding";

export type JsonRpcId = number | string;

export interface JsonRpcRequest {
  readonly jsonrpc: "2.0";
  readonly id: JsonRpcId;
  readonly method: string;
  readonly params: Json;
}

export interface JsonRpcSuccessResponse<T> {
  readonly jsonrpc: "2.0";
  readonly id: JsonRpcId;
  readonly result: T;
  readonly error: undefined;
}

export interface JsonRpcError {
  readonly code: number;
  readonly message: string;
  readonly data?: Json;
}

export interface JsonRpcErrorResponse {
  readonly jsonrpc: "2.0";
  readonly id: JsonRpcId | null;
  readonly error: JsonRpcError;
  readonly result: undefined;
}

export type JsonRpcResponse<T> = JsonRpcSuccessResponse<T> | JsonRpcErrorResponse;

export interface RpcClient {
  readonly request: <T>(method: string, params: Json) => Promise<JsonRpcResponse<T>>;
}

export interface JsonRpcBatchOptions {
  /** Interval for dispatching batches (in milliseconds) */
  readonly maxWait: number;
  /** Max number of items sent in one request */
  readonly maxSize: number;
}
