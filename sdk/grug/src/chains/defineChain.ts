import type { Chain, Json } from "@left-curve/types";

export function defineChain<extraFields extends Json, const chain extends Chain<extraFields>>(
  chain: chain,
): Chain<extraFields> {
  return {
    ...chain,
  };
}
