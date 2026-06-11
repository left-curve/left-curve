import { describe, expect, it } from "vitest";

import {
  getDefaultTradePairSymbols,
  getPerpsPairId,
  getPerpsPairLabel,
  getPerpsPairSymbol,
  getPerpsPairTicker,
  getTradeQuoteDenom,
  normalizeTradePairSymbols,
  parsePerpsPairId,
  parseTradePairSymbols,
} from "./tradePairSymbols";

describe("trade pair symbols", () => {
  it("uses the environment-specific default pair", () => {
    expect(getDefaultTradePairSymbols("Devnet")).toBe("ETH-USD");
    expect(getDefaultTradePairSymbols("Mainnet")).toBe("BTC-USD");
  });

  it("parses single-symbol URLs into USD pairs", () => {
    expect(parseTradePairSymbols("ETH")).toEqual({ baseSymbol: "ETH", quoteSymbol: "USD" });
    expect(parseTradePairSymbols("btc")).toEqual({ baseSymbol: "BTC", quoteSymbol: "USD" });
  });

  it("parses explicit pairs and rejects malformed input", () => {
    expect(parseTradePairSymbols("eth-usd")).toEqual({ baseSymbol: "ETH", quoteSymbol: "USD" });
    expect(parseTradePairSymbols("-USD")).toBeNull();
    expect(parseTradePairSymbols("ETH-USD-EXTRA")).toBeNull();
  });

  it("normalizes parsed symbols back into a canonical string", () => {
    expect(normalizeTradePairSymbols("ETH")).toBe("ETH-USD");
    expect(normalizeTradePairSymbols("eth-usd")).toBe("ETH-USD");
    expect(normalizeTradePairSymbols("-USD")).toBeNull();
  });

  it("derives perps pair ids from parsed symbols", () => {
    expect(getPerpsPairId({ baseSymbol: "ETH", quoteSymbol: "USD" })).toBe("perp/ethusd");
    expect(getPerpsPairId({ baseSymbol: "BTC", quoteSymbol: "USDC" })).toBe("perp/btcusdc");
  });

  it("round-trips perps pair ids through parsed symbols", () => {
    const symbols = { baseSymbol: "BTC", quoteSymbol: "USDC" };

    expect(parsePerpsPairId(getPerpsPairId(symbols))).toEqual(symbols);
  });

  it("derives base symbols from perps pair ids", () => {
    expect(getPerpsPairSymbol("perp/ethusd")).toBe("ETH");
    expect(getPerpsPairSymbol("perp/btcusd")).toBe("BTC");
    expect(getPerpsPairSymbol("perp/btcusdc")).toBe("BTC");
  });

  it("formats perps pair labels and tickers", () => {
    expect(getPerpsPairLabel("perp/ethusd")).toBe("ETH/USD");
    expect(getPerpsPairLabel("perp/btcusdc")).toBe("BTC/USDC");
    expect(getPerpsPairTicker("perp/ethusd")).toBe("ETHUSD");
    expect(getPerpsPairTicker("perp/btcusdc")).toBe("BTCUSDC");
  });

  it("uses the synthetic USD quote denom for perps", () => {
    expect(getTradeQuoteDenom("USD", {})).toBe("usd");
    expect(getTradeQuoteDenom("USDC", { USDC: { denom: "bridge/usdc" } })).toBe("bridge/usdc");
  });
});
