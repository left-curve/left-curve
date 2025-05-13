import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import type { GetConnectorClientReturnType } from "../actions/getConnectorClient.js";
import { useConnectorClient } from "./useConnectorClient.js";
import { useSessionKey } from "./useSessionKey.js";

type UseSigningClientReturnType = UseQueryResult<GetConnectorClientReturnType, Error>;

export function useSigningClient(): UseSigningClientReturnType {
  const { data: connectorClient } = useConnectorClient();
  const { client } = useSessionKey();

  return useQuery({
    enabled: Boolean(client) || Boolean(connectorClient),
    queryKey: ["signing_client", connectorClient?.uid, client?.type],
    queryFn: async () => {
      if (!client) return connectorClient;
      return client;
    },
  });
}
