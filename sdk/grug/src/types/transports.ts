import type { Chain } from "./chain.js";
import type { CometBftRpcSchema } from "./cometbft.js";
import type { Prettify } from "./utils.js";

export type TransportSchema = readonly {
  Method: string;
  Parameters?: unknown | undefined;
  ReturnType?: unknown;
}[];

export type TransportSchemaOverride = Omit<TransportSchema[number], "Method">;

export type RequestFnParameters<transportSchema extends TransportSchema | undefined = undefined> =
  transportSchema extends TransportSchema
    ? {
        [K in keyof transportSchema]: Prettify<
          {
            method: transportSchema[K] extends transportSchema[number]
              ? transportSchema[K]["Method"]
              : never;
          } & (transportSchema[K] extends transportSchema[number]
            ? transportSchema[K]["Parameters"] extends undefined
              ? { params?: undefined }
              : { params: transportSchema[K]["Parameters"] }
            : never)
        >;
      }[number]
    : {
        method: string;
        params?: unknown | undefined;
      };

export type DerivedTransportSchema<
  transportSchema extends TransportSchema | undefined,
  transportSchemaOverride extends TransportSchemaOverride | undefined,
> = transportSchemaOverride extends TransportSchemaOverride
  ? [transportSchemaOverride & { Method: string }]
  : transportSchema;

export type RequestFn<transportSchema extends TransportSchema | undefined = undefined> = <
  transportSchemaOverride extends TransportSchemaOverride | undefined = undefined,
  _parameters extends RequestFnParameters<
    DerivedTransportSchema<transportSchema, transportSchemaOverride>
  > = RequestFnParameters<DerivedTransportSchema<transportSchema, transportSchemaOverride>>,
  _returnType = DerivedTransportSchema<
    transportSchema,
    transportSchemaOverride
  > extends TransportSchema
    ? Extract<
        DerivedTransportSchema<transportSchema, transportSchemaOverride>[number],
        { Method: _parameters["method"] }
      >["ReturnType"]
    : unknown,
>(
  args: _parameters,
  options?: RequestOptions,
) => Promise<_returnType>;

export type RequestOptions = {
  // Deduplicate in-flight requests.
  dedupe?: boolean | undefined;
  // The base delay (in ms) between retries.
  retryDelay?: number | undefined;
  // The max number of times to retry.
  retryCount?: number | undefined;
  /** Unique identifier for the request. */
  uid?: string | undefined;
};

export type TransportConfig<
  type extends string = string,
  transportSchema extends TransportSchema = CometBftRpcSchema,
> = {
  /** The name of the transport. */
  name: string;
  /** The key of the transport. */
  key: string;
  /** The type of the transport. */
  type: type;
  /** Indicates if the transport supports batch queries. */
  batch?: boolean;
  request: RequestFn<transportSchema>;
};

export type Transport<
  type extends string = string,
  transportSchema extends TransportSchema = CometBftRpcSchema,
> = <chain extends Chain | undefined = Chain>(
  parameters: { chain?: chain | undefined } | undefined,
) => {
  config: TransportConfig<type, transportSchema>;
  request: RequestFn<transportSchema>;
};
