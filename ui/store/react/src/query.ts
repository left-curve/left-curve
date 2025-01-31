import {
  type DefaultError,
  type QueryKey,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
  useQuery as tanstack_useQuery,
  useMutation,
} from "@tanstack/react-query";

import type { ExactPartial, Prettify, UnionStrictOmit } from "@left-curve/dango/types";

export type UseMutationParameters<
  data = unknown,
  error = Error,
  variables = void,
  context = unknown,
> = Prettify<
  Omit<
    UseMutationOptions<data, error, Prettify<variables>, context>,
    "mutationFn" | "mutationKey" | "throwOnError"
  >
>;

export type UseMutationReturnType<
  data = unknown,
  error = Error,
  variables = void,
  context = unknown,
> = Prettify<
  UnionStrictOmit<UseMutationResult<data, error, variables, context>, "mutate" | "mutateAsync">
>;

export { useMutation };

////////////////////////////////////////////////////////////////////////////////

export type UseQueryParameters<
  queryFnData = unknown,
  error = DefaultError,
  data = queryFnData,
  queryKey extends QueryKey = QueryKey,
> = Prettify<
  ExactPartial<Omit<UseQueryOptions<queryFnData, error, data, queryKey>, "initialData">> & {
    initialData?: UseQueryOptions<queryFnData, error, data, queryKey>["initialData"] | undefined;
  }
>;

export type UseQueryReturnType<data = unknown, error = DefaultError> = Prettify<
  UseQueryResult<data, error> & {
    queryKey: QueryKey;
  }
>;

export type QueryParameter<
  queryFnData = unknown,
  error = DefaultError,
  data = queryFnData,
  queryKey extends QueryKey = QueryKey,
> = {
  query?:
    | Omit<
        UseQueryParameters<queryFnData, error, data, queryKey>,
        "queryFn" | "queryHash" | "queryKey" | "queryKeyHashFn" | "throwOnError"
      >
    | undefined;
};

export function useQuery<queryFnData, error, data, queryKey extends QueryKey>(
  parameters: UseQueryParameters<queryFnData, error, data, queryKey> & {
    queryKey: QueryKey;
  },
): UseQueryReturnType<data, error> {
  const result = tanstack_useQuery({
    ...parameters,
  }) as UseQueryReturnType<data, error>;
  result.queryKey = parameters.queryKey;
  return result;
}
