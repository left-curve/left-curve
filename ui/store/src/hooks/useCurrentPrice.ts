import { useLivePerpsTrades } from "./useLivePerpsTrades.js";

export type UseCurrentPriceParameters = {
  perpsPairId?: string;
  enabled?: boolean;
};

export function useCurrentPrice(parameters: UseCurrentPriceParameters) {
  return useLivePerpsTrades(
    (state) => ({
      currentPrice: state.currentPrice,
      previousPrice: state.previousPrice,
    }),
    parameters,
    (previous, next) =>
      previous.currentPrice === next.currentPrice && previous.previousPrice === next.previousPrice,
  );
}
