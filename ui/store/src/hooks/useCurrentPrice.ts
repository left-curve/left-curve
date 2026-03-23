import { orderBookStore } from "./useOrderBookState.js";
import { perpsOrderBookStore } from "./usePerpsOrderBookState.js";
import { tradePairStore } from "../stores/tradePairStore.js";

export function useCurrentPrice() {
  const mode = tradePairStore((s) => s.mode);

  const spotCurrent = orderBookStore((s) => s.currentPrice);
  const spotPrevious = orderBookStore((s) => s.previousPrice);

  const perpsCurrent = perpsOrderBookStore((s) => s.currentPrice);
  const perpsPrevious = perpsOrderBookStore((s) => s.previousPrice);

  return mode === "perps"
    ? { currentPrice: perpsCurrent, previousPrice: perpsPrevious }
    : { currentPrice: spotCurrent, previousPrice: spotPrevious };
}
