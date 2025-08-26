import { Direction } from "#types/dex.js";
import { Decimal } from "@left-curve/sdk/utils";

import type { Trade } from "#types/dex.js";

export function calculateTradeSize(trade: Trade, decimals: number) {
  if (trade.direction === Direction.Buy) {
    return Decimal(trade.refundBase).div(Decimal(10).pow(decimals));
  }

  return Decimal(trade.refundQuote).div(trade.clearingPrice).div(Decimal(10).pow(decimals));
}
