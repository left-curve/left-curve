import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "./usePublicClient.js";

import type { PerpsParam } from "@left-curve/dango/types";

export type UsePerpsParamParameters = {
  enabled?: boolean;
};

export function usePerpsParam(parameters?: UsePerpsParamParameters) {
  const { enabled = true } = parameters ?? {};
  const client = usePublicClient();

  return useQuery({
    enabled,
    queryKey: ["perps_param"],
    queryFn: async (): Promise<PerpsParam> => {
      return await client.getPerpsParam();
    },
    staleTime: 60 * 1000, // Params don't change often, cache for 1 minute
  });
}
