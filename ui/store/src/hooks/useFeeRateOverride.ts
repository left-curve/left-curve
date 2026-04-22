import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "./usePublicClient.js";
import { useAccount } from "./useAccount.js";

import type { FeeRateOverride } from "@left-curve/dango/types";

export type UseFeeRateOverrideParameters = {
  enabled?: boolean;
};

export function useFeeRateOverride(parameters?: UseFeeRateOverrideParameters) {
  const { enabled = true } = parameters ?? {};
  const client = usePublicClient();
  const { account, isConnected } = useAccount();

  const { data, isLoading, isError, error } = useQuery<FeeRateOverride | null>({
    queryKey: ["feeRateOverride", account?.address],
    queryFn: () => client.getFeeRateOverride({ user: account!.address }),
    enabled: enabled && isConnected && !!account?.address && !!client,
  });

  return {
    override: data,
    hasOverride: data != null,
    isLoading,
    isError,
    error,
  };
}
