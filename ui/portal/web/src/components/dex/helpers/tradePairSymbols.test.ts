import { describe, expect, it } from "vitest";

import {
  getDefaultTradePairSymbols,
  getPerpsPairIdFromSymbols,
  getTradeQuoteDenom,
  normalizeTradePairSymbols,
  parseTradePairSymbols,
} from "./tradePairSymbols";

describe("trade pair symbols", () => {
  it("uses the environment-specific default pair", () => {
    expect(getDefaultTradePairSymbols("Devnet")).toBe("ETH-USD");
    expect(getDefaultTradePairSymbols("Mainnet")).toBe("BTC-USD");
  });

  it("normalizes single-symbol URLs to USD pairs", () => {
    expect(normalizeTradePairSymbols("ETH")).toBe("ETH-USD");
    expect(normalizeTradePairSymbols("btc")).toBe("BTC-USD");
  });

  it("normalizes explicit pairs and rejects malformed pairs", () => {
    expect(normalizeTradePairSymbols("eth-usd")).toBe("ETH-USD");
    expect(normalizeTradePairSymbols("-USD")).toBeNull();
    expect(normalizeTradePairSymbols("ETH-USD-EXTRA")).toBeNull();
  });

  it("derives perps pair ids from normalized symbols", () => {
    expect(getPerpsPairIdFromSymbols("ETH")).toBe("perp/ethusd");
    expect(getPerpsPairIdFromSymbols("BTC-USDC")).toBe("perp/btcusdc");
    expect(getPerpsPairIdFromSymbols("BTC-USD-EXTRA")).toBeNull();
  });

  it("parses normalized symbols for UI denom lookup", () => {
    expect(parseTradePairSymbols("eth")).toEqual({ baseSymbol: "ETH", quoteSymbol: "USD" });
  });

  it("uses the synthetic USD quote denom for perps", () => {
    expect(getTradeQuoteDenom("USD", {})).toBe("usd");
    expect(getTradeQuoteDenom("USDC", { USDC: { denom: "bridge/usdc" } })).toBe("bridge/usdc");
  });
});
