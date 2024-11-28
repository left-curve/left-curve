"use client";

import { useEffect } from "react";

import {
  type ConnectData,
  type ConnectErrorType,
  type ConnectMutate,
  type ConnectMutateAsync,
  type ConnectVariables,
  connectMutationOptions,
} from "@left-curve/connect-kit/handlers";

import {
  type Config,
  type ConfigParameter,
  ConnectionStatus,
  type Prettify,
} from "@left-curve/types";
import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";

import { useConfig } from "./useConfig.js";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors.js";

export type UseConnectParameters<config extends Config = Config, context = unknown> = Prettify<
  ConfigParameter<config> & {
    mutation?: UseMutationParameters<ConnectData, ConnectErrorType, ConnectVariables, context>;
  }
>;

export type UseConnectReturnType<context = unknown> = Prettify<
  UseMutationReturnType<ConnectData, ConnectErrorType, ConnectVariables, context> & {
    connect: ConnectMutate<context>;
    connectAsync: ConnectMutateAsync<context>;
    connectors: Prettify<UseConnectorsReturnType>;
  }
>;

export function useConnect<config extends Config = Config, context = unknown>(
  parameters: UseConnectParameters<config, context> = {},
): UseConnectReturnType<context> {
  const { mutation } = parameters;

  const config = useConfig(parameters);

  const mutationOptions = connectMutationOptions(config);
  const { mutate, mutateAsync, ...result } = useMutation<
    ConnectData,
    ConnectErrorType,
    ConnectVariables,
    context
  >({
    ...mutation,
    ...mutationOptions,
  });

  useEffect(() => {
    return config.subscribe(
      ({ status }) => status,
      (status, previousStatus) => {
        if (
          previousStatus === ConnectionStatus.Connected &&
          status === ConnectionStatus.Disconnected
        )
          result.reset();
      },
    );
  }, [config, result.reset]);

  return {
    ...result,
    connect: mutate,
    connectAsync: mutateAsync,
    connectors: useConnectors({ config }),
  };
}
