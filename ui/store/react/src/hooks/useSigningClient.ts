import { useConnectorClient } from "@left-curve/store-react";
import { useQuery } from "@tanstack/react-query";
import { useSessionKey } from "./useSessionKey.js";

export function useSigningClient() {
  const { data: connectorClient } = useConnectorClient();
  const { client } = useSessionKey();

  return useQuery({
    queryKey: ["signing_client", connectorClient?.uid, client?.type],
    queryFn: async () => {
      if (!client) return connectorClient;
      return client;
    },
  });
}
