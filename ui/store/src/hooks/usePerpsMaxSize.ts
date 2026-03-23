import { useMemo } from "react";

type UsePerpsMaxSizeParameters = {
  availableMargin: number;
  leverage: number;
  currentPrice: number;
  isBaseSize: boolean;
};

export function usePerpsMaxSize(parameters: UsePerpsMaxSizeParameters) {
  const { availableMargin, leverage, currentPrice, isBaseSize } = parameters;

  return useMemo(() => {
    if (availableMargin <= 0 || currentPrice <= 0) return 0;
    const maxNotional = availableMargin * leverage;
    return isBaseSize ? maxNotional / currentPrice : maxNotional;
  }, [availableMargin, leverage, currentPrice, isBaseSize]);
}
