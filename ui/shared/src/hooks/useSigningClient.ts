import { useConnectorClient } from "@left-curve/react";
import { useQuery } from "@tanstack/react-query";
import { useSessionKey } from "./useSessionKey";

export function useSigningClient() {
  const { data: connectorClient } = useConnectorClient();
  const { client } = useSessionKey();

  return useQuery({
    queryKey: ["signing_client", connectorClient, client],
    queryFn: async () => {
      if (!client) return connectorClient;
      return client;
    },
  });
}
