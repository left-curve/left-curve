import {
  type ConnectErrorType,
  type ConnectParameters,
  type ConnectReturnType,
  connect,
} from "../actions/connect.js";

import type { Config } from "../types/store.js";
import type { Mutate, MutateAsync, MutationOptions } from "./mutation.js";
export type { ConnectErrorType } from "../actions/connect.js";

export function connectMutationOptions<config extends Config>(config: config) {
  return {
    mutationFn(variables) {
      return connect(config, variables);
    },
    mutationKey: ["connect"],
  } as const satisfies MutationOptions<ConnectData, ConnectErrorType, ConnectVariables>;
}

export type ConnectData = ConnectReturnType;

export type ConnectVariables = ConnectParameters;

export type ConnectMutate<context = unknown> = Mutate<
  ConnectData,
  ConnectErrorType,
  ConnectVariables,
  context
>;

export type ConnectMutateAsync<context = unknown> = MutateAsync<
  ConnectData,
  ConnectErrorType,
  ConnectVariables,
  context
>;
