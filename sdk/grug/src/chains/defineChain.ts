import type { Chain, Json } from "../types/index.js";

export function defineChain<extraFields extends Json, const chain extends Chain<extraFields>>(
  chain: chain,
): Chain<extraFields> {
  return {
    ...chain,
  };
}
