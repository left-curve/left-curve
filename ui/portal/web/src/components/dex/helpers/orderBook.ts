import { Decimal } from "@left-curve/utils";

import type { PerpsLiquidityDepthResponse } from "@left-curve/types";

const ROUND_DOWN = 0;
const ROUND_UP = 3;

type SnapDirection = "down" | "up";

type TopOfBookPrices = {
  bestAsk: string;
  bestBid: string;
};

export function getTopOfBookPrices(
  depth: Pick<PerpsLiquidityDepthResponse, "asks" | "bids"> | null,
): TopOfBookPrices | null {
  if (!depth) return null;

  const bidPrices = Object.keys(depth.bids);
  const askPrices = Object.keys(depth.asks);
  if (!bidPrices.length || !askPrices.length) return null;

  return {
    bestBid: bidPrices.reduce((max, price) => (Decimal(price).gt(max) ? price : max), bidPrices[0]),
    bestAsk: askPrices.reduce((min, price) => (Decimal(price).lt(min) ? price : min), askPrices[0]),
  };
}

export function getTopOfBookMidPrice(
  depth: Pick<PerpsLiquidityDepthResponse, "asks" | "bids"> | null,
  options: {
    snapDirection?: SnapDirection;
    tickSize?: string;
  } = {},
): string | null {
  const prices = getTopOfBookPrices(depth);
  if (!prices) return null;

  const midPrice = Decimal(prices.bestBid).plus(prices.bestAsk).div(2);
  if (midPrice.lte(0)) return null;

  const tickSize = options.tickSize ? Decimal(options.tickSize) : null;

  if (!tickSize || tickSize.lte(0) || !options.snapDirection) return midPrice.toFixed();

  const roundingMode = options.snapDirection === "down" ? ROUND_DOWN : ROUND_UP;
  return midPrice.div(tickSize).round(0, roundingMode).mul(tickSize).toFixed();
}
