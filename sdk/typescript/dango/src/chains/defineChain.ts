import type { Chain } from "@left-curve/types";

export function defineChain(config: Chain): Chain {
  return { ...config };
}
