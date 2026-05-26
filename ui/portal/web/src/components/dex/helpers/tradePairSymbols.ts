const DEFAULT_QUOTE_SYMBOL = "USD";
const DEFAULT_DEVNET_PAIR_SYMBOLS = `ETH-${DEFAULT_QUOTE_SYMBOL}`;
const DEFAULT_PAIR_SYMBOLS = `BTC-${DEFAULT_QUOTE_SYMBOL}`;

export type TradePairSymbols = {
  baseSymbol: string;
  quoteSymbol: string;
};

export function getDefaultTradePairSymbols(chainName: string): string {
  return chainName === "Devnet" ? DEFAULT_DEVNET_PAIR_SYMBOLS : DEFAULT_PAIR_SYMBOLS;
}

export function parseTradePairSymbols(pairSymbols: string): TradePairSymbols | null {
  const [rawBase, rawQuote, ...extra] = pairSymbols.split("-");
  if (extra.length > 0) return null;

  const baseSymbol = rawBase?.trim().toUpperCase();
  if (!baseSymbol) return null;

  const quoteSymbol = rawQuote?.trim().toUpperCase() || DEFAULT_QUOTE_SYMBOL;
  return { baseSymbol, quoteSymbol };
}

export function normalizeTradePairSymbols(pairSymbols: string): string | null {
  const parsed = parseTradePairSymbols(pairSymbols);
  return parsed && `${parsed.baseSymbol}-${parsed.quoteSymbol}`;
}

export function getPerpsPairId({ baseSymbol, quoteSymbol }: TradePairSymbols): string {
  return `perp/${baseSymbol.toLowerCase()}${quoteSymbol.toLowerCase()}`;
}

export function getTradeQuoteDenom(
  quoteSymbol: string,
  bySymbol: Record<string, { denom: string }>,
) {
  return quoteSymbol === DEFAULT_QUOTE_SYMBOL ? "usd" : bySymbol[quoteSymbol]?.denom;
}
