import { coins } from "./coins";

import type { BaseCoin } from "@left-curve/store/types";

export type MarketPairType = "crypto" | "commodity";

type PairDefinition = {
  id: string;
  base: BaseCoin;
  logoURI: string;
  name: string;
  quote: BaseCoin;
  ticker: string;
  type: MarketPairType;
};

const USD: BaseCoin = Object.freeze({
  decimals: 6,
  denom: "usd",
  logoURI: "/images/coins/usd.svg",
  name: "US Dollar",
  symbol: "USD",
});

// Keep this catalog in lockstep with backend perps listings before enabling new pairs.
const pairs = [
  {
    id: "perp/btcusd",
    base: coins["bridge/btc"],
    logoURI: "/images/coins/bitcoin.svg",
    name: "Bitcoin Perpetual",
    quote: USD,
    ticker: "BTCUSD",
    type: "crypto",
  },
  {
    id: "perp/ethusd",
    base: coins["bridge/eth"],
    logoURI: "/images/coins/eth.svg",
    name: "Ether Perpetual",
    quote: USD,
    ticker: "ETHUSD",
    type: "crypto",
  },
  {
    id: "perp/solusd",
    base: coins["bridge/sol"],
    logoURI: "/images/coins/sol.svg",
    name: "Solana Perpetual",
    quote: USD,
    ticker: "SOLUSD",
    type: "crypto",
  },
  {
    id: "perp/xrpusd",
    base: coins["bridge/xrp"],
    logoURI: "/images/coins/xrp.svg",
    name: "XRP Perpetual",
    quote: USD,
    ticker: "XRPUSD",
    type: "crypto",
  },
  {
    id: "perp/hypeusd",
    base: coins["bridge/hype"],
    logoURI: "/images/coins/hype.svg",
    name: "Hype Perpetual",
    quote: USD,
    ticker: "HYPEUSD",
    type: "crypto",
  },
  {
    id: "perp/xauusd",
    base: coins["perp/xauusd"],
    logoURI: "/images/coins/gold.svg",
    name: "Gold Perpetual",
    quote: USD,
    ticker: "XAUUSD",
    type: "commodity",
  },
  {
    id: "perp/xagusd",
    base: coins["perp/xagusd"],
    logoURI: "/images/coins/silver.svg",
    name: "Silver Perpetual",
    quote: USD,
    ticker: "XAGUSD",
    type: "commodity",
  },
  {
    id: "perp/wtiusd",
    base: coins["perp/wtiusd"],
    logoURI: "/images/coins/wtioil.svg",
    name: "WTI Oil Perpetual",
    quote: USD,
    ticker: "WTIUSD",
    type: "commodity",
  },
  {
    id: "perp/brentusd",
    base: coins["perp/brentusd"],
    logoURI: "/images/coins/brentoil.svg",
    name: "Brent Oil Perpetual",
    quote: USD,
    ticker: "BRENTUSD",
    type: "commodity",
  },
] as const satisfies readonly PairDefinition[];

const DEFAULT_TICKER = "BTCUSD";

function normalizeTicker(rawTicker: string): string | null {
  const ticker = rawTicker.trim().toUpperCase();
  return ticker ? ticker : null;
}

export class MarketPair {
  static readonly USD: BaseCoin = USD;
  static readonly quote: BaseCoin = USD;

  static readonly all: readonly MarketPair[] = Object.freeze(
    pairs.map((definition) => new MarketPair(definition)),
  );

  private static readonly byTicker: Readonly<Record<string, MarketPair>> = Object.freeze(
    Object.fromEntries(MarketPair.all.map((pair) => [pair.ticker, pair])),
  );

  private static readonly byId: Readonly<Record<string, MarketPair>> = Object.freeze(
    Object.fromEntries(MarketPair.all.map((pair) => [pair.id, pair])),
  );

  static readonly default: MarketPair = MarketPair.fromTicker(DEFAULT_TICKER);

  readonly id: string;
  readonly ticker: string;
  readonly name: string;
  readonly logoURI: string;
  readonly type: MarketPairType;
  readonly base: BaseCoin;
  readonly quote: BaseCoin;

  private constructor(definition: PairDefinition) {
    this.id = definition.id;
    this.ticker = definition.ticker;
    this.name = definition.name;
    this.logoURI = definition.logoURI;
    this.type = definition.type;
    this.base = definition.base;
    this.quote = definition.quote;

    Object.freeze(this);
  }

  static tryFromTicker(rawTicker: string): MarketPair | null {
    const ticker = normalizeTicker(rawTicker);
    if (!ticker) return null;

    return MarketPair.byTicker[ticker] ?? null;
  }

  static fromTicker(ticker: string): MarketPair {
    const pair = MarketPair.tryFromTicker(ticker);
    if (!pair) throw new Error(`Unknown pair ticker: ${ticker}`);

    return pair;
  }

  static tryFromPairId(pairId: string): MarketPair | null {
    return MarketPair.byId[pairId] ?? null;
  }

  static fromPairId(pairId: string): MarketPair {
    const pair = MarketPair.tryFromPairId(pairId);
    if (!pair) throw new Error(`Unknown pair id: ${pairId}`);

    return pair;
  }
}
