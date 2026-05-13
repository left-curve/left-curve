import type { Chain } from "../types/index.js";

export function defineChain(config: Chain): Chain {
  return { ...config };
}
