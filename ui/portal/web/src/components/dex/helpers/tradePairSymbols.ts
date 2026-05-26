const DEFAULT_QUOTE_SYMBOL = "USD";
const DEFAULT_DEVNET_PAIR_SYMBOLS = `ETH-${DEFAULT_QUOTE_SYMBOL}`;
const DEFAULT_PAIR_SYMBOLS = `BTC-${DEFAULT_QUOTE_SYMBOL}`;

type TradePairSymbols = {
  baseSymbol: string;
  quoteSymbol: string;
};

export function getDefaultTradePairSymbols(chainName: string): string {
  return chainName === "Devnet" ? DEFAULT_DEVNET_PAIR_SYMBOLS : DEFAULT_PAIR_SYMBOLS;
}

export function normalizeTradePairSymbols(pairSymbols: string): string | null {
  const [rawBaseSymbol, rawQuoteSymbol, ...extraSymbols] = pairSymbols.split("-");
  const baseSymbol = rawBaseSymbol?.trim().toUpperCase();
  const quoteSymbol = rawQuoteSymbol?.trim().toUpperCase() || DEFAULT_QUOTE_SYMBOL;

  if (!baseSymbol || extraSymbols.length > 0) return null;

  return `${baseSymbol}-${quoteSymbol}`;
}

export function parseTradePairSymbols(pairSymbols: string): TradePairSymbols | null {
  const normalizedPairSymbols = normalizeTradePairSymbols(pairSymbols);
  if (!normalizedPairSymbols) return null;

  const [baseSymbol = "", quoteSymbol = DEFAULT_QUOTE_SYMBOL] = normalizedPairSymbols.split("-");
  return { baseSymbol, quoteSymbol };
}

export function getPerpsPairIdFromSymbols(pairSymbols: string): string | null {
  const symbols = parseTradePairSymbols(pairSymbols);
  if (!symbols) return null;

  return `perp/${symbols.baseSymbol.toLowerCase()}${symbols.quoteSymbol.toLowerCase()}`;
}

export function getTradeQuoteDenom(
  quoteSymbol: string,
  bySymbol: Record<string, { denom: string }>,
) {
  return quoteSymbol === DEFAULT_QUOTE_SYMBOL ? "usd" : bySymbol[quoteSymbol]?.denom;
}
