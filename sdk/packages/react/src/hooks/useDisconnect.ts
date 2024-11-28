import {
  type DisconnectData,
  type DisconnectErrorType,
  type DisconnectMutate,
  type DisconnectMutateAsync,
  type DisconnectVariables,
  disconnectMutationOptions,
} from "@left-curve/connect-kit/handlers";
import type { ConfigParameter, Connector, Prettify } from "@left-curve/types";
import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";
import { useConfig } from "./useConfig.js";
import { useConnectors } from "./useConnectors.js";

export type UseDisconnectParameters<context = unknown> = Prettify<
  ConfigParameter & {
    mutation?:
      | UseMutationParameters<DisconnectData, DisconnectErrorType, DisconnectVariables, context>
      | undefined;
  }
>;

export type UseDisconnectReturnType<context = unknown> = Prettify<
  UseMutationReturnType<DisconnectData, DisconnectErrorType, DisconnectVariables, context> & {
    connectors: readonly Connector[];
    disconnect: DisconnectMutate<context>;
    disconnectAsync: DisconnectMutateAsync<context>;
  }
>;

export function useDisconnect<context = unknown>(
  parameters: UseDisconnectParameters<context> = {},
): UseDisconnectReturnType<context> {
  const { mutation } = parameters;

  const config = useConfig(parameters);

  const mutationOptions = disconnectMutationOptions(config);
  const { mutate, mutateAsync, ...result } = useMutation<
    DisconnectData,
    DisconnectErrorType,
    DisconnectVariables,
    context
  >({
    ...mutation,
    ...mutationOptions,
  });

  return {
    ...result,
    connectors: useConnectors({ config }),
    disconnect: mutate,
    disconnectAsync: mutateAsync,
  };
}
