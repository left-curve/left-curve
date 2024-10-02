"use client";

import { useEffect } from "react";

import {
  type ConnectData,
  type ConnectErrorType,
  type ConnectMutate,
  type ConnectMutateAsync,
  type ConnectVariables,
  connectMutationOptions,
} from "@leftcurve/connect-kit/handlers";

import type { Config, ConfigParameter, Prettify } from "@leftcurve/types";
import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query";

import { useConfig } from "./useConfig";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors";

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
  const { mutate, mutateAsync, ...result } = useMutation({
    ...mutation,
    ...mutationOptions,
  });

  useEffect(() => {
    return config.subscribe(
      ({ status }) => status,
      (status, previousStatus) => {
        if (previousStatus === "connected" && status === "disconnected") result.reset();
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
