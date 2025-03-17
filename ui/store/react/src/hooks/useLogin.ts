import { type UseMutationParameters, type UseMutationReturnType, useMutation } from "../query.js";
import { useChainId } from "./useChainId.js";
import { type UseConnectorsReturnType, useConnectors } from "./useConnectors.js";

export type UseLoginParameters = {
  username: string;
  connectors?: UseConnectorsReturnType;
  mutation?: UseMutationParameters<void, Error, { connectorId: string }>;
};

export type UseLoginReturnType = UseMutationReturnType<void, Error, { connectorId: string }>;

export function useLogin(parameters: UseLoginParameters) {
  const { username, mutation } = parameters;
  const connectors = parameters?.connectors ?? useConnectors();
  const chainId = useChainId();

  return useMutation({
    mutationFn: async ({ connectorId }) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");

      await connector.connect({
        username,
        chainId,
        challenge: "Please sign this message to confirm your identity.",
      });
    },
    ...mutation,
  });
}
