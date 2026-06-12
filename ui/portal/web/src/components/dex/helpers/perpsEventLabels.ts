import { m } from "@left-curve/foundation/paraglide/messages.js";

const PERPS_EVENT_LABELS: Record<string, () => string> = {
  order_filled: m["dex.protrade.tradeHistory.eventType.trade"],
  liquidated: m["dex.protrade.tradeHistory.eventType.liquidation"],
  deleveraged: m["dex.protrade.tradeHistory.eventType.adl"],
};

export function getPerpsEventLabel(eventType: string): string {
  return PERPS_EVENT_LABELS[eventType]?.() ?? eventType;
}

export function getSideLabel(isShort: boolean): string {
  return isShort
    ? m["dex.protrade.tradeHistory.side.sell"]()
    : m["dex.protrade.tradeHistory.side.buy"]();
}

export function getMakerTakerLabel(isMaker: boolean): string {
  return isMaker ? m["dex.protrade.tradeHistory.maker"]() : m["dex.protrade.tradeHistory.taker"]();
}
