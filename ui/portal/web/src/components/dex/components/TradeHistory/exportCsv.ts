import { calculateTradeSize, Decimal } from "@left-curve/dango/utils";
import { Direction, TimeInForceOption } from "@left-curve/dango/types";

import type {
  DeleveragedData,
  LiquidatedData,
  OrderFilledData,
  PerpsEvent,
  Trade,
} from "@left-curve/dango/types";

const PERPS_EVENT_LABELS: Record<string, string> = {
  order_filled: "Trade",
  liquidated: "Liquidation",
  deleveraged: "ADL",
};

type CoinInfo = { symbol: string; decimals: number };
type CoinsByDenom = Record<string, CoinInfo | undefined>;

export type PerpsCsvHeaders = {
  pair: string;
  type: string;
  direction: string;
  size: string;
  tradeValue: string;
  price: string;
  pnl: string;
  funding: string;
  fees: string;
  makerTaker: string;
  time: string;
};

export type SpotCsvHeaders = {
  pair: string;
  direction: string;
  type: string;
  size: string;
  price: string;
  time: string;
};

function csvEscape(value: string | null | undefined): string {
  if (value === null || value === undefined) return "";
  const str = String(value);
  if (/[",\n\r]/.test(str)) return `"${str.replace(/"/g, '""')}"`;
  return str;
}

function rowsToCsv(headers: readonly string[], rows: readonly (readonly string[])[]): string {
  const lines = [headers.map(csvEscape).join(",")];
  for (const row of rows) lines.push(row.map(csvEscape).join(","));
  return lines.join("\n");
}

function normalizePerpsEvent(event: PerpsEvent) {
  switch (event.eventType) {
    case "order_filled": {
      const d = event.data as OrderFilledData;
      return {
        size: d.fill_size,
        price: d.fill_price,
        pnl: d.realized_pnl,
        fee: d.fee,
        funding: d.realized_funding,
        isMaker: d.is_maker,
      };
    }
    case "liquidated": {
      const d = event.data as LiquidatedData;
      return {
        size: d.adl_size,
        price: d.adl_price,
        pnl: d.adl_realized_pnl,
        fee: null,
        funding: d.adl_realized_funding,
        isMaker: null,
      };
    }
    case "deleveraged": {
      const d = event.data as DeleveragedData;
      return {
        size: d.closing_size,
        price: d.fill_price,
        pnl: d.realized_pnl,
        fee: null,
        funding: d.realized_funding,
        isMaker: null,
      };
    }
    default:
      return {
        size: null,
        price: null,
        pnl: null,
        fee: null,
        funding: null,
        isMaker: null,
      };
  }
}

export function buildPerpsTradeHistoryCsv(
  events: readonly PerpsEvent[],
  headers: PerpsCsvHeaders,
): string {
  const headerRow = [
    headers.pair,
    headers.type,
    headers.direction,
    headers.size,
    headers.tradeValue,
    headers.price,
    headers.pnl,
    headers.funding,
    headers.fees,
    headers.makerTaker,
    headers.time,
  ];

  const rows: string[][] = events.map((event) => {
    const norm = normalizePerpsEvent(event);
    const pair = event.pairId.replace("perp/", "").replace(/usd$/i, "/USD").toUpperCase();
    const baseSymbol = event.pairId.replace("perp/", "").replace(/usd$/i, "").toUpperCase();
    const eventLabel = PERPS_EVENT_LABELS[event.eventType] ?? event.eventType;

    const isShort = norm.size?.startsWith("-") ?? false;
    const direction = norm.size ? (isShort ? "Sell" : "Buy") : "";
    const absSize = norm.size ? (isShort ? norm.size.slice(1) : norm.size) : "";
    const sizeWithSymbol = absSize ? `${absSize} ${baseSymbol}` : "";

    const tradeValue =
      absSize && norm.price ? Decimal(absSize).times(Decimal(norm.price)).toFixed() : "";

    const makerTaker =
      event.eventType !== "order_filled" || norm.isMaker === null
        ? ""
        : norm.isMaker
          ? "Maker"
          : "Taker";

    return [
      pair,
      eventLabel,
      direction,
      sizeWithSymbol,
      tradeValue,
      norm.price ?? "",
      norm.pnl ?? "",
      norm.funding ?? "",
      norm.fee ?? "",
      makerTaker,
      event.createdAt,
    ];
  });

  return rowsToCsv(headerRow, rows);
}

export function buildSpotTradeHistoryCsv(
  trades: readonly Trade[],
  coinsByDenom: CoinsByDenom,
  headers: SpotCsvHeaders,
): string {
  const headerRow = [
    headers.pair,
    headers.direction,
    headers.type,
    headers.size,
    headers.price,
    headers.time,
  ];

  const rows: string[][] = trades.map((trade) => {
    const baseInfo = coinsByDenom[trade.baseDenom];
    const quoteInfo = coinsByDenom[trade.quoteDenom];
    const baseSymbol = baseInfo?.symbol ?? trade.baseDenom;
    const quoteSymbol = quoteInfo?.symbol ?? trade.quoteDenom;
    const baseDecimals = baseInfo?.decimals ?? 6;
    const quoteDecimals = quoteInfo?.decimals ?? 6;

    const size = calculateTradeSize(trade, baseDecimals).toFixed();
    const price = Decimal(trade.clearingPrice)
      .times(Decimal(10).pow(baseDecimals - quoteDecimals))
      .toFixed();
    const direction = trade.direction === Direction.Buy ? "Buy" : "Sell";
    const orderType = trade.timeInForce === TimeInForceOption.GoodTilCanceled ? "Limit" : "Market";

    return [`${baseSymbol}/${quoteSymbol}`, direction, orderType, size, price, trade.createdAt];
  });

  return rowsToCsv(headerRow, rows);
}

export function downloadCsv(filename: string, content: string): void {
  // Prefix with BOM so Excel detects UTF-8 properly.
  const blob = new Blob([`\ufeff${content}`], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.style.display = "none";
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

export function tradeHistoryCsvFilename(market: "perps" | "spot"): string {
  const date = new Date().toISOString().slice(0, 10);
  return `trade-history-${market}-${date}.csv`;
}
