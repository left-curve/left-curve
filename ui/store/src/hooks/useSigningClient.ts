import { useQuery } from "@tanstack/react-query";
import { useConnectorClient } from "./useConnectorClient.js";
import { useSessionKey } from "./useSessionKey.js";

export function useSigningClient() {
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
