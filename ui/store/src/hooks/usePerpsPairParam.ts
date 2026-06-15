import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "./usePublicClient.js";

import type { PerpsPairParam } from "@left-curve/types";

export type UsePerpsPairParamParameters = {
  pairId: string;
  enabled?: boolean;
};

export function usePerpsPairParam(parameters: UsePerpsPairParamParameters) {
  const { pairId, enabled = true } = parameters;
  const client = usePublicClient();

  return useQuery({
    enabled,
    queryKey: ["perps_pair_param", pairId],
    queryFn: async (): Promise<PerpsPairParam | null> => {
      return await client.getPerpsPairParam({ pairId });
    },
    staleTime: 60 * 1000, // Params don't change often, cache for 1 minute
  });
}
