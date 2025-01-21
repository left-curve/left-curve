import {
  type DisconnectErrorType,
  type DisconnectParameters,
  type DisconnectReturnType,
  disconnect,
} from "../actions/disconnect.js";

import type { Config } from "../types/store.js";
import type { Mutate, MutateAsync, MutationOptions } from "./mutation.js";
export type { DisconnectErrorType } from "../actions/disconnect.js";

export function disconnectMutationOptions<config extends Config>(config: config) {
  return {
    mutationFn(variables) {
      return disconnect(config, variables);
    },
    mutationKey: ["disconnect"],
  } as const satisfies MutationOptions<DisconnectData, DisconnectErrorType, DisconnectVariables>;
}

export type DisconnectData = DisconnectReturnType;

export type DisconnectVariables = DisconnectParameters;

export type DisconnectMutate<context = unknown> = Mutate<
  DisconnectData,
  DisconnectErrorType,
  DisconnectVariables,
  context
>;

export type DisconnectMutateAsync<context = unknown> = MutateAsync<
  DisconnectData,
  DisconnectErrorType,
  DisconnectVariables,
  context
>;
