import type { Json } from "./encoding";
import type { HttpRequestParameters } from "./http";
import type { Prettify } from "./utils";

export type RpcSchema = readonly {
  Method: string;
  Parameters?: unknown | undefined;
  ReturnType?: unknown;
}[];

type RpcSchemaOverride = Omit<RpcSchema[number], "Method">;

export type RequestFnParameters<rpcSchema extends RpcSchema | undefined = undefined> =
  rpcSchema extends RpcSchema
    ? {
        [K in keyof rpcSchema]: Prettify<
          {
            method: rpcSchema[K] extends rpcSchema[number] ? rpcSchema[K]["Method"] : never;
          } & (rpcSchema[K] extends rpcSchema[number]
            ? rpcSchema[K]["Parameters"] extends undefined
              ? { params?: undefined }
              : { params: rpcSchema[K]["Parameters"] }
            : never)
        >;
      }[number]
    : {
        method: string;
        params?: unknown | undefined;
      };

export type DerivedRpcSchema<
  rpcSchema extends RpcSchema | undefined,
  rpcSchemaOverride extends RpcSchemaOverride | undefined,
> = rpcSchemaOverride extends RpcSchemaOverride
  ? [rpcSchemaOverride & { Method: string }]
  : rpcSchema;

export type RequestFn<rpcSchema extends RpcSchema | undefined = undefined> = <
  rpcSchemaOverride extends RpcSchemaOverride | undefined = undefined,
  _parameters extends RequestFnParameters<
    DerivedRpcSchema<rpcSchema, rpcSchemaOverride>
  > = RequestFnParameters<DerivedRpcSchema<rpcSchema, rpcSchemaOverride>>,
  _returnType = DerivedRpcSchema<rpcSchema, rpcSchemaOverride> extends RpcSchema
    ? Extract<
        DerivedRpcSchema<rpcSchema, rpcSchemaOverride>[number],
        { Method: _parameters["method"] }
      >["ReturnType"]
    : unknown,
>(
  args: _parameters,
  options?: RpcRequestOptions,
) => Promise<_returnType>;

export type RpcRequestOptions = {
  // Deduplicate in-flight requests.
  dedupe?: boolean | undefined;
  // The base delay (in ms) between retries.
  retryDelay?: number | undefined;
  // The max number of times to retry.
  retryCount?: number | undefined;
  /** Unique identifier for the request. */
  uid?: string | undefined;
};

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
