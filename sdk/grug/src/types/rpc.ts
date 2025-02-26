import type { Json } from "./encoding.js";
import type { HttpRequestParameters } from "./http.js";

export type JsonRpcId = number;

export type JsonRpcRequest = {
  readonly jsonrpc: "2.0";
  readonly id: JsonRpcId;
  readonly method: string;
  readonly params: Json;
};

export type JsonRpcSuccessResponse<T> = {
  readonly jsonrpc: "2.0";
  readonly id: JsonRpcId;
  readonly result: T;
  readonly error: undefined;
};

export type JsonRpcError = {
  readonly code: number;
  readonly message: string;
  readonly data?: Json;
};

export type JsonRpcErrorResponse = {
  readonly jsonrpc: "2.0";
  readonly id: JsonRpcId;
  readonly error: JsonRpcError;
  readonly result: undefined;
};

export type JsonRpcResponse<T = any> = JsonRpcSuccessResponse<T> | JsonRpcErrorResponse;

export type RpcClient = {
  readonly request: <body extends JsonRpcRequest | JsonRpcRequest[]>(
    params: HttpRequestParameters<body>,
  ) => Promise<body extends JsonRpcRequest[] ? JsonRpcResponse[] : JsonRpcResponse>;
};

export interface JsonRpcBatchOptions {
  /** Interval for dispatching batches (in milliseconds) */
  readonly maxWait: number;
  /** Max number of items sent in one request */
  readonly maxSize: number;
}
