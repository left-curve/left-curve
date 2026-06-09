import { Decimal } from "@left-curve/utils";

import type { OrderFilledData, PerpsEvent } from "@left-curve/types";

const BUY_COLOR = "#27AE60";
const SELL_COLOR = "#EB5757";
const LABEL_COLOR = "#FFFCF6";

export type FillMarkerSide = "buy" | "sell";

export type FillMarker = {
  id: string;
  time: number;
  color: {
    border: string;
    background: string;
  };
  text: string;
  label: "B" | "S";
  labelFontColor: string;
  minSize: number;
};

type BuildFillMarkerParameters = {
  resolution: string;
};

function resolutionToMilliseconds(resolution: string): number | undefined {
  if (resolution.includes("S")) {
    const seconds = Number.parseInt(resolution, 10);
    return (Number.isNaN(seconds) ? 1 : seconds) * 1_000;
  }

  if (resolution.includes("W")) return 7 * 24 * 60 * 60 * 1_000;
  if (resolution.includes("D")) return 24 * 60 * 60 * 1_000;

  const minutes = Number.parseInt(resolution, 10);
  if (Number.isNaN(minutes)) return undefined;
  return minutes * 60 * 1_000;
}

export function getFillMarkerBarTime(fillTimeMs: number, resolution: string): number | undefined {
  const intervalMs = resolutionToMilliseconds(resolution);
  if (!intervalMs || !Number.isFinite(fillTimeMs)) {
    return undefined;
  }

  if (resolution.includes("W")) {
    const fillDate = new Date(fillTimeMs);
    const daysSinceSunday = fillDate.getUTCDay();
    const dayStartMs = Date.UTC(
      fillDate.getUTCFullYear(),
      fillDate.getUTCMonth(),
      fillDate.getUTCDate(),
    );
    return Math.floor((dayStartMs - daysSinceSunday * 24 * 60 * 60 * 1_000) / 1_000);
  }

  return Math.floor(Math.floor(fillTimeMs / intervalMs) * intervalMs) / 1_000;
}

function getPairSymbol(pairId: string): string {
  return pairId.replace("perp/", "").replace("usd", "").toUpperCase();
}

function shortHash(hash: string): string {
  if (hash.length <= 14) return hash;
  return `${hash.slice(0, 8)}...${hash.slice(-6)}`;
}

function buildMarkerText(
  event: PerpsEvent,
  data: OrderFilledData,
  side: FillMarkerSide,
  absSize: string,
  price: string,
): string[] {
  const pairSymbol = getPairSymbol(event.pairId);
  const sideLabel = side === "buy" ? "Buy" : "Sell";
  const text = [
    `${sideLabel} ${absSize} ${pairSymbol} at $${price}`,
    `Fee: $${data.fee}`,
    `Realized PnL: ${data.realized_pnl}`,
  ];

  if (data.realized_funding !== undefined && data.realized_funding !== null) {
    text.push(`Funding: ${data.realized_funding}`);
  }

  if (data.is_maker !== undefined && data.is_maker !== null) {
    text.push(data.is_maker ? "Maker" : "Taker");
  }

  text.push(`Time: ${event.createdAt}`);
  if (event.txHash) text.push(`Tx: ${shortHash(event.txHash)}`);

  return text;
}

export function buildFillMarker(
  event: PerpsEvent,
  parameters: BuildFillMarkerParameters,
): FillMarker | null {
  if (event.eventType !== "order_filled") return null;

  const data = event.data as OrderFilledData;
  const fillTimeMs = Date.parse(event.createdAt);
  if (!Number.isFinite(fillTimeMs)) return null;

  try {
    const size = Decimal(data.fill_size);
    const price = Decimal(data.fill_price);
    const priceNumber = price.toNumber();
    if (size.isZero() || !Number.isFinite(priceNumber)) return null;

    const time = getFillMarkerBarTime(fillTimeMs, parameters.resolution);
    if (time === undefined) return null;

    const side: FillMarkerSide = size.gt(0) ? "buy" : "sell";
    const color = side === "buy" ? BUY_COLOR : SELL_COLOR;
    const absSize = size.abs().toFixed();
    const fillPrice = price.toFixed();

    return {
      id: `${event.txHash}:${event.idx}:${data.order_id}:${data.fill_id ?? ""}`,
      time,
      color: {
        border: color,
        background: color,
      },
      text: buildMarkerText(event, data, side, absSize, fillPrice).join("\n"),
      label: side === "buy" ? "B" : "S",
      labelFontColor: LABEL_COLOR,
      minSize: 16,
    };
  } catch {
    return null;
  }
}
