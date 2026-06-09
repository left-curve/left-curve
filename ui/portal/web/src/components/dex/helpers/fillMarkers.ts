import { Decimal, truncateAddress } from "@left-curve/utils";
import { getChartResolutionBarTime } from "./chartResolution";
import { normalizePerpsEvent, type NormalizedFields } from "./normalizePerpsEvent";
import { getPerpsPairSymbol } from "./tradePairSymbols";

import type { PerpsEvent } from "@left-curve/types";

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

type DecimalValue = ReturnType<typeof Decimal>;

type MarkerTextValues = {
  fields: NormalizedFields;
  side: FillMarkerSide;
  size: DecimalValue;
  price: DecimalValue;
};

function buildMarkerText(event: PerpsEvent, values: MarkerTextValues): string[] {
  const { fields, side, size, price } = values;
  const pairSymbol = getPerpsPairSymbol(event.pairId);
  const sideLabel = side === "buy" ? "Buy" : "Sell";
  const text = [`${sideLabel} ${size.abs().toFixed()} ${pairSymbol} at $${price.toFixed()}`];

  if (fields.fee !== undefined) {
    text.push(`Fee: $${fields.fee}`);
  }

  if (fields.pnl !== undefined) {
    text.push(`Realized PnL: ${fields.pnl}`);
  }

  if (fields.funding !== undefined) {
    text.push(`Funding: ${fields.funding}`);
  }

  if (fields.isMaker !== undefined) {
    text.push(fields.isMaker ? "Maker" : "Taker");
  }

  text.push(`Time: ${event.createdAt}`);
  if (event.txHash) text.push(`Tx: ${truncateAddress(event.txHash, 6)}`);

  return text;
}

export function buildFillMarker(
  event: PerpsEvent,
  parameters: BuildFillMarkerParameters,
): FillMarker | null {
  if (event.eventType !== "order_filled") return null;

  const fields = normalizePerpsEvent(event);
  const fillTimeMs = Date.parse(event.createdAt);
  if (!Number.isFinite(fillTimeMs)) return null;

  if (!fields.size || !fields.price) return null;

  const size = Decimal(fields.size);
  const price = Decimal(fields.price);
  if (size.isZero() || !Number.isFinite(price.toNumber())) return null;

  const time = getChartResolutionBarTime(fillTimeMs, parameters.resolution);
  if (time === undefined) return null;

  const side: FillMarkerSide = size.gt(0) ? "buy" : "sell";
  const color = side === "buy" ? BUY_COLOR : SELL_COLOR;

  return {
    id: `${event.txHash}:${event.idx}`,
    time,
    color: {
      border: color,
      background: color,
    },
    text: buildMarkerText(event, { fields, side, size, price }).join("\n"),
    label: side === "buy" ? "B" : "S",
    labelFontColor: LABEL_COLOR,
    minSize: 16,
  };
}
