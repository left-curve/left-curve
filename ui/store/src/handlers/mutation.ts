import type { MutateOptions } from "@tanstack/query-core";

export type { MutationOptions } from "@tanstack/query-core";

import type { Prettify } from "@left-curve/dango/types";

export type Mutate<data = unknown, error = unknown, variables = void, context = unknown> = (
  ...args: Parameters<MutateFn<data, error, Prettify<variables>, context>>
) => void;

export type MutateAsync<
  data = unknown,
  error = unknown,
  variables = void,
  context = unknown,
> = MutateFn<data, error, Prettify<variables>, context>;

type MutateFn<
  data = unknown,
  error = unknown,
  variables = void,
  context = unknown,
> = undefined extends variables
  ? (
      variables?: variables,
      options?: Prettify<MutateOptions<data, error, variables, context>> | undefined,
    ) => Promise<data>
  : (
      variables: variables,
      options?: Prettify<MutateOptions<data, error, variables, context>> | undefined,
    ) => Promise<data>;
