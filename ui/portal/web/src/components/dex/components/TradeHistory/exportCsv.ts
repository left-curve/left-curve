import { Decimal } from "@left-curve/utils";

import type { PerpsEvent } from "@left-curve/types";

import { normalizePerpsEvent } from "../../helpers/normalizePerpsEvent";
import { getPerpsPairLabel, getPerpsPairSymbol } from "../../helpers/tradePairSymbols";
import { getMakerTakerLabel, getPerpsEventLabel, getSideLabel } from "./perpsEventLabels";

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
    const baseSymbol = getPerpsPairSymbol(event.pairId);
    const pair = getPerpsPairLabel(event.pairId);
    const eventLabel = getPerpsEventLabel(event.eventType);

    const isShort = norm.size?.startsWith("-") ?? false;
    const direction = norm.size ? getSideLabel(isShort) : "";
    const absSize = norm.size ? (isShort ? norm.size.slice(1) : norm.size) : "";
    const sizeWithSymbol = absSize ? `${absSize} ${baseSymbol}` : "";

    const tradeValue =
      absSize && norm.price ? Decimal(absSize).times(Decimal(norm.price)).toFixed() : "";

    const makerTaker =
      event.eventType !== "order_filled" || norm.isMaker === undefined
        ? ""
        : getMakerTakerLabel(norm.isMaker);

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

export function downloadCsv(filename: string, content: string): void {
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

export function tradeHistoryCsvFilename(): string {
  const date = new Date().toISOString().slice(0, 10);
  return `trade-history-perps-${date}.csv`;
}
