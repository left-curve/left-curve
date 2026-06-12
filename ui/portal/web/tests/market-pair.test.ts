import { describe, expect, it } from "vitest";

import { MarketPair } from "@left-curve/foundation/market-pair";

describe("MarketPair", () => {
  it("resolves cataloged ticker input to market metadata", () => {
    expect(MarketPair.tryFromTicker("ethusd")?.id).toBe("perp/ethusd");
    expect(MarketPair.tryFromTicker(" eth ")).toBeNull();
    expect(MarketPair.tryFromTicker("ETHUSD")?.id).toBe("perp/ethusd");
    expect(MarketPair.tryFromTicker("ETH-USD")).toBeNull();
  });

  it("resolves backend pair ids and exposes product type metadata", () => {
    const gold = MarketPair.fromPairId("perp/xauusd");

    expect(gold.ticker).toBe("XAUUSD");
    expect("label" in gold).toBe(false);
    expect(gold.name).toBe("Gold Perpetual");
    expect(gold.logoURI).toBe("/images/coins/gold.svg");
    expect(gold.type).toBe("commodity");
  });

  it("fails fast for backend pair ids that are missing from the catalog", () => {
    expect(() => MarketPair.fromPairId("perp/atomusd")).toThrow("Unknown pair id: perp/atomusd");
  });

  it("keeps the route default in the catalog", () => {
    expect(MarketPair.default.ticker).toBe("BTCUSD");
    expect(MarketPair.default.id).toBe("perp/btcusd");
  });

  it("exposes base and quote asset metadata for UI consumers", () => {
    const eth = MarketPair.fromTicker("ETHUSD");

    expect("baseSymbol" in eth).toBe(false);
    expect("quoteSymbol" in eth).toBe(false);
    expect(eth.name).toBe("Ether Perpetual");
    expect(eth.logoURI).toBe("/images/coins/eth.svg");
    expect(eth.base).toEqual({
      decimals: 18,
      denom: "bridge/eth",
      logoURI: "/images/coins/eth.svg",
      name: "Ether",
      symbol: "ETH",
      type: "native",
    });
    expect(eth.quote).toBe(MarketPair.USD);
    expect(MarketPair.quote).toBe(MarketPair.USD);
    expect(MarketPair.USD).toEqual({
      decimals: 6,
      denom: "usd",
      logoURI: "/images/coins/usd.svg",
      name: "US Dollar",
      symbol: "USD",
    });
  });
});
