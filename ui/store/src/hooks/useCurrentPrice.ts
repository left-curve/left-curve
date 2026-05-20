import { livePerpsTradesStore } from "./useLivePerpsTradesState.js";

export function useCurrentPrice() {
  const currentPrice = livePerpsTradesStore((s) => s.currentPrice);
  const previousPrice = livePerpsTradesStore((s) => s.previousPrice);

  return { currentPrice, previousPrice };
}
